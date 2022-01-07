use crate::{Object, PolygonOperations, Settings, Slice};
use geo::prelude::*;
use geo::*;
use rayon::prelude::*;

pub trait ObjectPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings);
}

pub struct BrimPass {}

impl ObjectPass for BrimPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings) {
        if let Some(width) = &settings.brim_width {
            println!("Generating Moves: Brim");
            //Add to first object

            let first_layer_multipolygon: MultiPolygon<f64> = MultiPolygon(
                objects
                    .iter()
                    .map(|poly| {
                        let first_slice = poly.layers
                            .get(0)
                            .expect("Object needs a Slice");

                        first_slice.get_entire_slice_polygon()
                            .0
                            .clone()
                            .into_iter()
                            .chain(
                                first_slice.get_support_polygon().into_iter()
                            )
                    })
                    .flatten()
                    .collect(),
            );

            objects
                .get_mut(0)
                .expect("Needs an object")
                .layers
                .get_mut(0)
                .expect("Object needs a Slice")
                .generate_brim(first_layer_multipolygon, *width);
        }
    }
}

pub struct SupportTowerPass {}

impl ObjectPass for SupportTowerPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings) {
        if let Some(support) = &settings.support {
            println!("Generating Support Towers");
            //Add to first object

            objects
                .par_iter_mut()
                .for_each(|obj| {
                    (1..obj.layers.len()).into_iter().rev().for_each(|q| {
                        //todo Fix this, it feels hacky
                        if let [ref mut layer, ref mut above, ..] = &mut obj.layers[(q - 1..=q)] {
                            layer.add_support_polygons(&above, &support);
                        } else {
                            unreachable!()
                        }
                    });
                });

        }
    }
}

pub struct SkirtPass {}

impl ObjectPass for SkirtPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings) {
        //Handle Perimeters
        if let Some(skirt) = &settings.skirt {
            println!("Generating Moves: Skirt");
            let convex_hull = objects
                .iter()
                .map(|object| {
                    object
                        .layers
                        .iter()
                        .take(skirt.layers)
                        .map(|m| m.get_entire_slice_polygon().union_with(&m.get_support_polygon()))
                })
                .flatten()
                .fold(
                    MultiPolygon(vec![]),
                    |a, b| a.union_with(&b),
                )
                .convex_hull();

            //Add to first object
            objects
                .get_mut(0)
                .expect("Needs an object")
                .layers
                .iter_mut()
                .take(skirt.layers)
                .enumerate()
                .for_each(|(_layer_num, slice)| slice.generate_skirt(&convex_hull, skirt))
        }
    }
}

pub trait SlicePass {
    fn pass(slices: &mut Vec<Slice>, settings: &Settings);
}

pub struct PerimeterPass {}

impl SlicePass for PerimeterPass {
    fn pass(slices: &mut Vec<Slice>, settings: &Settings) {
        println!("Generating Moves: Perimeters");
        slices
            .par_iter_mut()
            .enumerate()
            .for_each(|(_layer_num, slice)| {
                slice.slice_perimeters_into_chains(settings.number_of_perimeters);
            });
    }
}

pub struct BridgingPass {}

impl SlicePass for BridgingPass {
    fn pass(slices: &mut Vec<Slice>, _settings: &Settings) {
        println!("Generating Moves: Bridging");
        (1..slices.len()).into_iter().for_each(|q| {
            let below = slices[q - 1].get_entire_slice_polygon().clone();

            slices[q].fill_solid_bridge_area(&below);
        });
    }
}
pub struct TopLayerPass {}

impl SlicePass for TopLayerPass {
    fn pass(slices: &mut Vec<Slice>, _settings: &Settings) {
        println!("Generating Moves: Top Layer");
        (0..slices.len() - 1).into_iter().for_each(|q| {
            let above = slices[q + 1].get_entire_slice_polygon().clone();

            slices[q].fill_solid_top_layer(&above, q);
        });
    }
}

pub struct TopAndBottomLayersPass {}

impl SlicePass for TopAndBottomLayersPass {
    fn pass(slices: &mut Vec<Slice>, settings: &Settings) {
        let top_layers = settings.top_layers;
        let bottom_layers = settings.bottom_layers;

        //Make sure at least 1 layer will not be solid
        if slices.len() > bottom_layers + top_layers {
            println!("Generating Moves: Above and below support");

            (bottom_layers..slices.len() - top_layers)
                .into_iter()
                .for_each(|q| {
                    let below = if bottom_layers != 0 {
                        Some(
                            slices[(q - bottom_layers + 1)..q]
                                .iter()
                                .map(|m| m.get_entire_slice_polygon())
                                .fold(
                                    slices
                                        .get(q - bottom_layers)
                                        .expect("Bounds Checked above")
                                        .get_entire_slice_polygon()
                                        .clone(),
                                    |a, b| a.intersection_with(b),
                                ),
                        )
                    } else {
                        None
                    };
                    let above = if top_layers != 0 {
                        Some(
                            slices[q + 1..q + top_layers + 1]
                                .iter()
                                .map(|m| m.get_entire_slice_polygon())
                                .fold(
                                    slices
                                        .get(q + 1)
                                        .expect("Bounds Checked above")
                                        .get_entire_slice_polygon()
                                        .clone(),
                                    |a, b| a.intersection_with(b),
                                ),
                        )
                    } else {
                        None
                    };
                    if let Some(intersection) = match (above, below) {
                        (None, None) => None,
                        (None, Some(poly)) | (Some(poly), None) => Some(poly),
                        (Some(polya), Some(polyb)) => Some(polya.intersection_with(&polyb)),
                    } {
                        slices
                            .get_mut(q)
                            .expect("Bounds Checked above")
                            .fill_solid_subtracted_area(&intersection, q);
                    }
                });
        }
    }
}

pub struct SupportPass {}

impl SlicePass for SupportPass {
    fn pass(slices: &mut Vec<Slice>, settings: &Settings) {
        if let Some(support) = &settings.support {

            for slice in slices.iter_mut() {
                slice.fill_support_polygons(&support);
            }
        }
    }
}

pub struct FillAreaPass {}

impl SlicePass for FillAreaPass {
    fn pass(slices: &mut Vec<Slice>, settings: &Settings) {
        println!("Generating Moves: Fill Areas");

        let slice_count = slices.len();

        //Fill all remaining areas
        slices
            .par_iter_mut()
            .enumerate()
            .for_each(|(layer_num, slice)| {
                slice.fill_remaining_area(
                    layer_num < settings.bottom_layers
                        || settings.top_layers + layer_num + 1 > slice_count,
                    layer_num,
                );
            });
    }
}

pub struct OrderPass {}

impl SlicePass for OrderPass {
    fn pass(slices: &mut Vec<Slice>, _settings: &Settings) {
        println!("Generating Moves: Order Chains");

        //Fill all remaining areas
        slices.par_iter_mut().for_each(|slice| {
            slice.order_chains();
        });
    }
}
