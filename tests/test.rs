
mod errors {
    error_chain::error_chain! { }
}
use errors::*;
use shapefile::{record::{traits::HasXY}, Shape};
#[link(
    name = "s2",
    kind = "dylib",
)]
extern {}
use autocxx::prelude::*;
include_cpp! {
    #include "cc/polygon.h"
    safety!(unsafe)
    generate!("Polygons")
    generate!("Polygon")
    generate!("LngLat")
    generate!("LngLats")
    generate!("Loop")
}
#[path = "../src/utils/mod.rs"]
mod utils;
use core::pin::Pin;
use rayon::prelude::*;

fn load_polygons(
    shapefile_path: String, 
    attributes: Vec<String>
) -> Result<Pin<Box<ffi::Polygons>>> {
    let mut polygons = ffi::Polygons::new().within_box();
    let shapes:(Vec<Shape>, Vec<Vec<String>>)
        = utils::shape::load_shape(&shapefile_path, &attributes).unwrap();
    shapes.0.iter().enumerate().for_each(|(shape_idx, shape)| {
        match shape {
            Shape::Polygon(gp) => {
                let name = shapes.1[shape_idx][1].as_str();
                let polygon 
                    = utils::shape::load_polygon(&shapefile_path, gp, name, 
                        false, "").unwrap();
                polygons.as_mut().add(polygon);
            },
            _ => { 
                panic!("Not polygon shape"); 
            }
        }
    });
    Ok(polygons)
}

#[test]
fn test_search_polygon() {
    // let attributes: Vec<String> = vec!["EMD_CD", "EMD_KOR_NM", "EMD_ENG_NM"]
    //     .iter().map(|&s| s.to_string()).collect();
    let attributes: Vec<String> = vec!["A3", "A4", "A5"]
        .iter().map(|&s| s.to_string()).collect();
    // let shapefile_path = "./data/all_converted/46000/TL_SCCO_EMD.shp";
    let shapefile_path = "./data/all_converted/41000/AL_D002_41_20250204_6_part3.shp";
    let shapes:(Vec<Shape>, Vec<Vec<String>>)
        = utils::shape::load_shape(shapefile_path, &attributes).unwrap();
    let mut polygons = ffi::Polygons::new().within_box();
    let debug = true;
    shapes.0.iter().enumerate().for_each(|(shape_idx, shape)| {
        match shape {
            Shape::Polygon(gp) => {
                let name = shapes.1[shape_idx][1].as_str();
                if debug {
                    println!("{} {} rings {:?}", shape_idx, name, gp.rings().len());
                }
                let polygon 
                    = utils::shape::load_polygon(shapefile_path, gp, name, 
                        debug, "").unwrap();
                polygons.as_mut().add(polygon);
            },
            _ => { 
                panic!("Not polygon shape"); 
            }
        }
    });

    let mut r = polygons.search(126.366999290, 34.768900882);
    assert_eq!(r, c_int(60));
    r = polygons.search(126.391191105, 34.778120908);
    assert_eq!(r, c_int(-1));
    r = polygons.search(126.361351688, 34.765351976);
    assert_eq!(r, c_int(-1));
    r = polygons.search(126.314839702, 34.772191620);
    assert_eq!(r, c_int(60));
    r = polygons.search(126.303433151, 34.768225399);
    assert_eq!(r, c_int(-1));
    r = polygons.search(126.363634356, 34.760158647);
    assert_eq!(r, c_int(60));
    r = polygons.search(126.359617547, 34.760827253);
    assert_eq!(r, c_int(-1));    
}

#[tokio::test]
async fn test_load_polygon() {
    let attributes: Vec<String> = vec!["A3", "A4", "A5"]
        .iter()
        .map(|&s| s.to_string())
        .collect();

    let shapefile_paths = vec![
        "./data/all_converted/41000/AL_D002_41_20250204_6_part3.shp",
        "./data/all_converted/41000/AL_D002_41_20250204_6_part2.shp",
        "./data/all_converted/41000/AL_D002_41_20250204_6_part1.shp",
        "./data/all_converted/41000/AL_D002_41_20250204_5_part21.shp",
        "./data/all_converted/41000/AL_D002_41_20250204_5_part22.shp",
        "./data/all_converted/41000/AL_D002_41_20250204_3_part10.shp",
        "./data/all_converted/41000/AL_D002_41_20250204_3_part11.shp",
    ];
  
    shapefile_paths
        .par_iter()
        .for_each(|path| {
            let attributes = attributes.clone();
            let path = path.to_string();
            let reuslt = load_polygons(path, attributes);     
        });
}

