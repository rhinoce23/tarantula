use shapefile::{record::{traits::HasXY}, Shape};
mod errors {
    error_chain::error_chain! { }
}
use errors::*;
use error_chain::bail;
use shapefile::dbase::FieldValue;
use crate::ffi;
use autocxx::prelude::*;
use std::pin::Pin;

pub fn load_shape<T>(file_path: &str, attributes: &Vec<String>) 
    -> Result<(Vec<T>, Vec<Vec<String>>)>
where
    T: From<Shape>,
{
    let mut reader = 
        shapefile::Reader::from_path(file_path).chain_err(|| format!("{}", file_path))?;
    let mut shapes = Vec::new();
    let mut shape_attributes = Vec::new();
    for shape_record in reader.iter_shapes_and_records() {
        let (shape, record) = shape_record.chain_err(|| "shape record")?;
        let mut attribute_iter = attributes.iter();
        let mut attributes = Vec::new();
        loop {
            match attribute_iter.next() {
                Some(attr) => {
                    let value = record.get(attr);
                    match value {
                        Some(value) => {
                            match value {
                                FieldValue::Character(value) => {
                                    if let Some(str) = value {
                                        attributes.push(str.to_string())
                                    } else {
                                        attributes.push("".to_string());
                                    }
                                },
                                _ => bail!("failed to get string"),
                            }
                        },
                        None => break,
                    }
                },
                None => break,
            }
        }

        if attributes.iter().all(|attr| attr.is_empty()) {
            bail!("failed to get attributes");
        }        

        shapes.push(T::from(shape));
        shape_attributes.push(attributes);
    } 

    Ok((shapes, shape_attributes))
}

fn is_same_lnglat(lnglat1: (f64, f64), lnglat2: (f64, f64)) -> bool {
    let (x_diff, y_diff) = (lnglat1.0 - lnglat2.0, lnglat1.1 - lnglat2.1);
    if x_diff.abs() <= 1e-7 && y_diff.abs() <= 1e-7 {
        return true;
    }
    false
}

pub fn load_polygon(shapefile: &str, gp: &shapefile::record::Polygon, name: &str, debug: bool,
    debug_name: &str) 
    -> Result<Pin<Box<ffi::Polygon>>> {
    let mut polygon = ffi::Polygon::new().within_box();
    gp.rings().iter().enumerate().for_each(|(ring_index, ring)| {
        
        let mut first = (0f64, 0f64);
        let mut pprev = (0f64, 0f64);
        let mut prev = (0f64, 0f64);
        let mut lnglats = ffi::LngLats::new().within_box();
        let mut ok = false;
        let mut lnglats_filter = vec![];
        ring.points().iter().enumerate().for_each(|(point_idx, point)| {
            ok = false;
            let lnglat = (point.x(), point.y());
            if point_idx == 0 {
                ok = true;
            } else {
                if !is_same_lnglat(lnglat, prev) {
                    ok = true;
                }
                if is_same_lnglat(lnglat, first) {
                    ok = false;
                }

                if is_same_lnglat(lnglat, pprev) {
                    ok = false;
                    lnglats.as_mut().pop_back();
                }
            }

            if ok && point_idx < ring.points().len() - 1 {
                pprev = prev;
                prev = (lnglat.0, lnglat.1);
                if point_idx == 0 {
                    first = prev;
                }
                
                let is_duplicate = lnglats_filter
                    .iter()
                    .any(|existing| is_same_lnglat(lnglat, *existing));
                if !is_duplicate {
                    lnglats.as_mut().add(lnglat.0, lnglat.1);
                    lnglats_filter.push(lnglat);
                    if name == debug_name {
                        println!("{} {} {} {} {:.7},{:.7}", 
                            shapefile, name, ring_index, lnglats.size(), lnglat.0, lnglat.1);
                    }
                }
            }
        });

        const MIN_POINTS: usize = 3;
        if lnglats.size() < MIN_POINTS {
            return;
        }

        let outer = match ring {
            shapefile::PolygonRing::Inner(_) => {
                false
            }   
            _ => {
                true
            }            
        };

        let mut aloop = ffi::Loop::new().within_box();
        let error_code = aloop.as_mut().init(lnglats, outer, debug) as i32;
        if error_code == 0 {
            polygon.as_mut().add(aloop);
        } else if error_code != 4 {
            if debug {
                panic!("{} load_polygon {} error_code: {}", shapefile, name, error_code);
            } else {
                println!("{} load_polygon {} error_code: {}", shapefile, name, error_code);
            }
        }
    });

    Ok(polygon)
}
