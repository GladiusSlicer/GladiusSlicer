use crate::plotter::lightning_infill::lightning_infill;
use crate::plotter::support::Supporter;
use crate::plotter::Plotter;
use crate::utils::display_state_update;
use crate::{Object, PolygonOperations, Settings, Slice};
use geo::prelude::*;
use geo::*;
use gladius_shared::error::SlicerErrors;
use gladius_shared::types::PartialInfillTypes;
use rayon::prelude::*;

pub trait ObjectPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings, send_messages: bool);
}

pub struct BrimPass {}

impl ObjectPass for BrimPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings, send_messages: bool) {
        if let Some(width) = &settings.brim_width {
            display_state_update("Generating Moves: Brim", send_messages);
            //Add to first object

            let first_layer_multipolygon: MultiPolygon<f64> = MultiPolygon(
                objects
                    .iter()
                    .flat_map(|poly| {
                        let first_slice = poly.layers.get(0).expect("Object needs a Slice");

                        first_slice
                            .main_polygon
                            .0
                            .clone()
                            .into_iter()
                            .chain(first_slice.main_polygon.clone().into_iter())
                    })
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
    fn pass(objects: &mut Vec<Object>, settings: &Settings, send_messages: bool) {
        if let Some(support) = &settings.support {
            display_state_update("Generating Support Towers", send_messages);
            //Add to first object

            objects.par_iter_mut().for_each(|obj| {
                (1..obj.layers.len()).into_iter().rev().for_each(|q| {
                    //todo Fix this, it feels hacky
                    if let [ref mut layer, ref mut above, ..] = &mut obj.layers[(q - 1..=q)] {
                        layer.add_support_polygons(above, support);
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
    fn pass(objects: &mut Vec<Object>, settings: &Settings, send_messages: bool) {
        //Handle Perimeters
        if let Some(skirt) = &settings.skirt {
            display_state_update("Generating Moves: Skirt", send_messages);
            let convex_hull = objects
                .iter()
                .flat_map(|object| {
                    object
                        .layers
                        .iter()
                        .take(skirt.layers)
                        .map(|m| m.main_polygon.union_with(&m.get_support_polygon()))
                })
                .fold(MultiPolygon(vec![]), |a, b| a.union_with(&b))
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
    fn pass(
        slices: &mut Vec<Slice>,
        settings: &Settings,
        send_message: bool,
    ) -> Result<(), SlicerErrors>;
}

pub struct ShrinkPass {}

impl SlicePass for ShrinkPass {
    fn pass(
        slices: &mut Vec<Slice>,
        _settings: &Settings,
        send_messages: bool,
    ) -> Result<(), SlicerErrors> {
        display_state_update("Generating Moves: Shrink Layers", send_messages);
        slices.par_iter_mut().for_each(|slice| {
            slice.shrink_layer();
        });
        Ok(())
    }
}

pub struct PerimeterPass {}

impl SlicePass for PerimeterPass {
    fn pass(
        slices: &mut Vec<Slice>,
        settings: &Settings,
        send_messages: bool,
    ) -> Result<(), SlicerErrors> {
        display_state_update("Generating Moves: Perimeters", send_messages);
        slices.par_iter_mut().for_each(|slice| {
            slice.slice_perimeters_into_chains(settings.number_of_perimeters);
        });
        Ok(())
    }
}

pub struct BridgingPass {}

impl SlicePass for BridgingPass {
    fn pass(
        slices: &mut Vec<Slice>,
        _settings: &Settings,
        send_messages: bool,
    ) -> Result<(), SlicerErrors> {
        display_state_update("Generating Moves: Bridging", send_messages);
        (1..slices.len()).into_iter().for_each(|q| {
            let below = slices[q - 1].main_polygon.clone();

            slices[q].fill_solid_bridge_area(&below);
        });
        Ok(())
    }
}
pub struct TopLayerPass {}

impl SlicePass for TopLayerPass {
    fn pass(
        slices: &mut Vec<Slice>,
        _settings: &Settings,
        send_messages: bool,
    ) -> Result<(), SlicerErrors> {
        display_state_update("Generating Moves: Top Layer", send_messages);
        (0..slices.len() - 1).into_iter().for_each(|q| {
            let above = slices[q + 1].main_polygon.clone();

            slices[q].fill_solid_top_layer(&above, q);
        });
        Ok(())
    }
}

pub struct TopAndBottomLayersPass {}

impl SlicePass for TopAndBottomLayersPass {
    fn pass(
        slices: &mut Vec<Slice>,
        settings: &Settings,
        send_messages: bool,
    ) -> Result<(), SlicerErrors> {
        let top_layers = settings.top_layers;
        let bottom_layers = settings.bottom_layers;

        //Make sure at least 1 layer will not be solid
        if slices.len() > bottom_layers + top_layers {
            display_state_update("Generating Moves: Above and below support", send_messages);

            (bottom_layers..slices.len() - top_layers)
                .into_iter()
                .for_each(|q| {
                    let below = if bottom_layers != 0 {
                        Some(
                            slices[(q - bottom_layers + 1)..q]
                                .iter()
                                .map(|m| m.main_polygon.clone())
                                .fold(
                                    slices
                                        .get(q - bottom_layers)
                                        .expect("Bounds Checked above")
                                        .main_polygon
                                        .clone(),
                                    |a, b| a.intersection_with(&b),
                                ),
                        )
                    } else {
                        None
                    };
                    let above = if top_layers != 0 {
                        Some(
                            slices[q + 1..q + top_layers + 1]
                                .iter()
                                .map(|m| m.main_polygon.clone())
                                .fold(
                                    slices
                                        .get(q + 1)
                                        .expect("Bounds Checked above")
                                        .main_polygon
                                        .clone(),
                                    |a, b| a.intersection_with(&b),
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

        let slice_count = slices.len();

        slices
            .par_iter_mut()
            .enumerate()
            .filter(|(layer_num, _)| {
                *layer_num < settings.bottom_layers
                    || settings.top_layers + *layer_num + 1 > slice_count
            })
            .for_each(|(layer_num, slice)| {
                slice.fill_remaining_area(true, layer_num);
            });
        Ok(())
    }
}

pub struct SupportPass {}

impl SlicePass for SupportPass {
    fn pass(
        slices: &mut Vec<Slice>,
        settings: &Settings,
        _send_messages: bool,
    ) -> Result<(), SlicerErrors> {
        if let Some(support) = &settings.support {
            for slice in slices.iter_mut() {
                slice.fill_support_polygons(support);
            }
        }
        Ok(())
    }
}

pub struct FillAreaPass {}

impl SlicePass for FillAreaPass {
    fn pass(
        slices: &mut Vec<Slice>,
        _settings: &Settings,
        send_messages: bool,
    ) -> Result<(), SlicerErrors> {
        display_state_update("Generating Moves: Fill Areas", send_messages);

        //Fill all remaining areas
        slices
            .par_iter_mut()
            .enumerate()
            .for_each(|(layer_num, slice)| {
                slice.fill_remaining_area(false, layer_num);
            });
        Ok(())
    }
}
pub struct LightningFillPass {}

impl SlicePass for LightningFillPass {
    fn pass(
        slices: &mut Vec<Slice>,
        settings: &Settings,
        send_messages: bool,
    ) -> Result<(), SlicerErrors> {
        if settings.partial_infill_type == PartialInfillTypes::Lightning {
            display_state_update("Generating Moves: Lightning Infill", send_messages);

            lightning_infill(slices);
        }
        Ok(())
    }
}

pub struct OrderPass {}

impl SlicePass for OrderPass {
    fn pass(
        slices: &mut Vec<Slice>,
        _settings: &Settings,
        send_messages: bool,
    ) -> Result<(), SlicerErrors> {
        display_state_update("Generating Moves: Order Chains", send_messages);

        //Fill all remaining areas
        slices.par_iter_mut().for_each(|slice| {
            slice.order_chains();
        });
        Ok(())
    }
}
