
mod errors {
    error_chain::error_chain! { }
}
use errors::*;
use scopeguard::defer;
use serde::{Deserialize, Serialize};
use crate::{ffi, config::Search as SearchConfig};
use autocxx::prelude::*;
use shapefile::{record::{traits::HasXY}, Shape};
#[path = "./utils/mod.rs"]
mod utils;
use core::pin::Pin;
use rayon::prelude::*;
use stopwatch::Stopwatch;
use std::collections::HashMap;
use std::sync::RwLock;
pub static mut GLOBAL_SEARCH: Option<Search> = None;
use std::sync::Mutex;

pub fn initialize_global_search() {
    unsafe {
        GLOBAL_SEARCH = Some(
            Search::new(crate::GLOBAL_CONFIG.search.clone())
                .and_then(|mut search| {
                    search.load().expect("failed to load search");
                    Ok(search)
                })
                .expect("failed to create search")
        );
    }
}

unsafe impl Send for ffi::Polygons {}
unsafe impl Sync for ffi::Polygons {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub district: String,
    pub level: i32,
    pub name: String,
    pub lnglats: Vec<(f64, f64)>,
}

#[derive(Debug, Clone)]
pub struct PolyInfo {
    pub district: String,
    pub level: i32,
    pub name: String,
}

pub struct Search {
    config: SearchConfig,
    hierarchies: Vec<(Pin<Box<ffi::Polygons>>, Vec<PolyInfo>)>,
    district_par: RwLock<HashMap<String, Vec<(Pin<Box<ffi::Polygons>>, Vec<PolyInfo>)>>>,
    district_par_any: RwLock<HashMap<String, Vec<(Pin<Box<ffi::Polygons>>, Vec<PolyInfo>)>>>,
}

impl Search{
    pub fn new(config: SearchConfig) -> Result<Self> {
        Ok(
            Self {
                config,
                hierarchies: vec![],
                district_par: RwLock::new(HashMap::new()),
                district_par_any: RwLock::new(HashMap::new()),
            }
        )
    }

    pub fn load(&mut self) -> Result<()> {
        let sw = Stopwatch::start_new();
        defer! {
            println!("Search load elapsed: {:?}", sw.elapsed());
        }

        let config = &self.config.clone();
       
        config.districts
            .iter()
            .for_each(|district| {
                if let Ok(mut district_par) 
                    = self.district_par.write() {
                        district_par.insert(district.to_string(), vec![]);     
                }
                if let Ok(mut district_par_any) 
                    = self.district_par_any.write() {
                        district_par_any.insert(district.to_string(), vec![]);     
                }
            });
    
        config.districts.iter().try_for_each(|district| -> Result<()> {
            config.hierarchies.iter().try_for_each(|name| {
                let attribute = config
                    .shapefile
                    .attributes
                    .get(name)
                    .chain_err(|| format!("{} attribute", name))?;
        
                let result = self.get_polygons(
                    district,
                    attribute.level,
                    format!("{}/{}/{}.shp", config.shapefile.path, district, name)
                    .as_str(),
                    &attribute.names.as_ref(),
                    config.debug,
                    &config.debug_name,
                )?;
        
                self.hierarchies.push(result);
                Ok(())
            })
        })?;

        config.districts
            .par_iter()
            .try_for_each(|district| {
                config.district_par
                    .par_iter()
                    .try_for_each(|name|
                        self.load_district_par_polygons(
                            config,
                            district, 
                            name,
                            config.debug, 
                            &config.debug_name,
                        )
                    )?;
                config.district_par_any
                    .par_iter()
                    .try_for_each(|name|
                        self.load_district_par_any_polygons(
                            config,
                            district, 
                            name,
                            config.debug, 
                            &config.debug_name,
                        )
                    )
            })?;

        // warm up the index
        let _ = self.search(127.1, 35.1);
        
        Ok(())
    }

    fn load_district_par_any_polygons(&self, config: &SearchConfig, district: &str,
        name: &str, debug: bool, debug_name: &str) -> Result<()> {
        let sw = Stopwatch::start_new();
        defer!({
            println!("load_district_par_any_polygons {} {} elapsed: {:?}", 
                district, name, sw.elapsed());
        });
       
        let file_pattern = format!(
            "{}/{}/{}*.shp",
            config.shapefile.path,
            district,
            name,
        );

        let paths: Vec<_> = glob::glob(&file_pattern)
            .chain_err(|| "failed to read glob pattern")?
            .collect();

        paths.par_iter().try_for_each(|entry| {
                match entry {
                    Ok(path) => {
                        if let Some(path) = path.to_str() {
                            if let Some(attribute) 
                                = config.shapefile.attributes.get(name) {  
                                if let Ok(result) = self.get_polygons(
                                    district, 
                                    attribute.level,
                                    path, 
                                    &attribute.names.as_ref(), 
                                     config.debug, 
                                    &config.debug_name,
                                ) {
                                    if let Ok(mut a) 
                                        = self.district_par_any.write() {
                                        if let Some(b) 
                                            = a.get_mut(district) {
                                            b.push(result);
                                        }
                                    } 
                                }
                            }
                        } 
                        Ok(())
                    }
                    Err(e) => {
                        return Err(format!("match enty {:?}", e));
                    }
                }
            })?;
        Ok(())
    }

    fn load_district_par_polygons(&self, config: &SearchConfig, district: &str,
        name: &str, debug: bool, debug_name: &str) -> Result<()> {
        let sw = Stopwatch::start_new();
        defer!({
            println!("load_district_polygons {} {} elapsed: {:?}", 
                district, name, sw.elapsed());
        });

        let path = format!("{}/{}/{}.shp", 
            config.shapefile.path, district, name);
        let attribute = config.shapefile.attributes.get(name)
            .chain_err(|| format!("failed to get attributes for {}", name))?;
        let result = self.get_polygons(
            district, 
            attribute.level,
            path.as_str(), 
            &attribute.names.as_ref(), 
            debug, 
            debug_name,
        ).chain_err(|| format!("failed to get polygons for {} {}", district, name))?;

        let mut a 
            = self.district_par.write().map_err(|_| "failed to lock write")?;
        let b 
            = a.get_mut(district).chain_err(|| 
                format!("failed to get polys_vec {}", district))?;
        
        b.push(result);

        Ok(())
    }

    fn get_polygons(&self, district: &str, level: i32, shapefile: &str, 
        attributes: &Vec<String>, debug: bool, debug_name: &str)
        -> Result<(Pin<Box<ffi::Polygons>>, Vec<PolyInfo>)> {
        let shapes: (Vec<Shape>, Vec<Vec<String>>) 
            = utils::shape::load_shape(shapefile, attributes)
                .chain_err(|| format!("{}", shapefile))?;
        let mut polys = ffi::Polygons::new().within_box();
        let mut polys_infos = vec![];
        shapes.0.iter().enumerate().for_each(|(shape_idx, shape)| {
            match shape {
                Shape::Polygon(gp) => {
                    let name = shapes.1[shape_idx][1].as_str();
                    let polygon 
                        = utils::shape::load_polygon(shapefile, gp, name, debug, debug_name);
                    match polygon {
                        Ok(polygon) => {
                            if debug {
                                println!("loading {} {} {} rings {:?}", 
                                    shapefile, name, shape_idx, gp.rings().len());
                            }

                            polys.as_mut().add(polygon);
                            polys_infos.push(
                                PolyInfo {
                                    district: district.to_string(),
                                    level,
                                    name: name.to_string(),
                                }
                            );
                        },
                        Err(e) => {
                            println!("loading error {} {} rings {:?}", 
                                name, shape_idx, gp.rings().len());
                        }
                    }
                },
                _ => {}
            }
        });
        Ok((polys, polys_infos))
    }

    pub fn search(&self, lon: f64, lat: f64) -> Result<Vec<Info>> {
        let debug = self.config.debug;
        let mut results = vec![];
        self.hierarchies.iter()
            .for_each(|polys| {
                let r = polys.0.search(lon, lat);
                let j = i32::from(r);
                if j >= 0 {
                    let info = &polys.1[j as usize];
                    if debug {         
                        println!("{:?}", info);
                    }
                    results.push(Info {
                        district: info.district.clone(),
                        level: info.level,
                        name: info.name.clone(),
                        lnglats: vec![],
                    });
                    if let Ok(a) 
                        = self.district_par.read() {
                        if let Some(b) 
                            = a.get(info.district.as_str()) {
                            let infos: Vec<Info> = b.par_iter()
                                .filter_map(|d| {
                                let r = d.0.search(lon, lat);
                                let j = i32::from(r);
                                if j >= 0 {
                                    let info = &d.1[j as usize];
                                    if debug {         
                                        println!("{:?}", info);
                                    }
                                    Some(Info {
                                        district: info.district.clone(),
                                        level: info.level,
                                        name: info.name.clone(),
                                        lnglats: vec![],
                                    })
                                } else {
                                    None
                                }
                            }).collect();
                            results.extend(infos);
                        }
                    }
                    if let Ok(a) 
                        = self.district_par_any.read() {
                        if let Some(b) 
                            = a.get(info.district.as_str()) {
                            let info = Mutex::new(None);   
                            let exist = b.par_iter().any(|d| {
                                let r = d.0.search_polygon(lon, lat);
                                let j = i32::from(r.index());
                                if j >= 0 {
                                    let a = &d.1[j as usize];
                                    if debug {         
                                        println!("{:?}", a);
                                    }
                                    if let Ok(mut b) 
                                        = info.lock() {
                                        *b = Some(Info {
                                            district: a.district.clone(),
                                            level: a.level,
                                            name: a.name.clone(),
                                            lnglats: r.lnglats()
                                                .iter()
                                                .map(|ll| (ll.lng(), ll.lat()))
                                                .collect(),
                                        });
                                    }
                                    true
                                } else {
                                    false
                                }
                            });
                            if exist {
                                if let Ok(mut a) 
                                    = info.lock() {
                                    if let Some(b) = a.take() {
                                        results.push(b.clone());
                                    }
                                } 
                            }
                        }
                    }
                }
            }
        );
        
        results.sort_by_key(|info| info.level);
        Ok(results)
    }
}