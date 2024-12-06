use crate::plotter::lightning_infill::lightning_infill;
use crate::plotter::support::Supporter;
use crate::plotter::Plotter;
use crate::utils::{state_update, StateContext};
use crate::{Object, PolygonOperations, Settings, Slice};
use geo::prelude::*;
use geo::MultiPolygon;
use gladius_shared::error::SlicerErrors;
use gladius_shared::types::PartialInfillTypes;
use rayon::prelude::*;

pub trait ObjectPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings, state_context: &mut StateContext);
}

pub struct BrimPass;

impl ObjectPass for BrimPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings, state_context: &mut StateContext) {
        if let Some(width) = &settings.brim_width {
            state_update("Generating Moves: Brim", state_context);

            // Add to first object
            let first_layer_multipolygon: MultiPolygon<f64> = MultiPolygon(
                objects
                    .iter()
                    .flat_map(|poly| {
                        let first_slice = poly.layers.first().expect("Object needs a Slice");

                        first_slice
                            .main_polygon
                            .0
                            .clone()
                            .into_iter()
                            .chain(first_slice.main_polygon.clone())
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

pub struct SupportTowerPass;

impl ObjectPass for SupportTowerPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings, state_context: &mut StateContext) {
        if let Some(support) = &settings.support {
            state_update("Generating Support Towers", state_context);
            // Add to first object

            objects.par_iter_mut().for_each(|obj| {
                (1..obj.layers.len()).rev().for_each(|q| {
                    // todo Fix this, it feels hacky
                    if let [ref mut layer, ref mut above, ..] = &mut obj.layers[q - 1..=q] {
                        layer.add_support_polygons(above, support);
                    } else {
                        unreachable!()
                    }
                });
            });
        }
    }
}

pub struct SkirtPass;

impl ObjectPass for SkirtPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings, state_context: &mut StateContext) {
        // Handle Perimeters
        if let Some(skirt) = &settings.skirt {
            state_update("Generating Moves: Skirt", state_context);
            let convex_hull = objects
                .iter()
                .flat_map(|object| {
                    object
                        .layers
                        .iter()
                        .take(skirt.layers as usize)
                        .map(|m| m.main_polygon.union_with(&m.get_support_polygon()))
                })
                .fold(MultiPolygon(vec![]), |a, b| a.union_with(&b))
                .convex_hull();

            // Add to first object
            objects
                .get_mut(0)
                .expect("Needs an object")
                .layers
                .iter_mut()
                .take(skirt.layers as usize)
                .for_each(|slice| slice.generate_skirt(&convex_hull, skirt, settings));
        }
    }
}

pub trait SlicePass {
    fn pass(
        slices: &mut Vec<Slice>,
        settings: &Settings,
        state_context: &mut StateContext,
    ) -> Result<(), SlicerErrors>;
}

pub struct ShrinkPass;

impl SlicePass for ShrinkPass {
    fn pass(
        slices: &mut Vec<Slice>,
        _settings: &Settings,
        state_context: &mut StateContext,
    ) -> Result<(), SlicerErrors> {
        state_update("Generating Moves: Shrink Layers", state_context);
        slices.par_iter_mut().for_each(|slice| {
            slice.shrink_layer();
        });
        Ok(())
    }
}

pub struct PerimeterPass;

impl SlicePass for PerimeterPass {
    fn pass(
        slices: &mut Vec<Slice>,
        settings: &Settings,
        state_context: &mut StateContext,
    ) -> Result<(), SlicerErrors> {
        state_update("Generating Moves: Perimeters", state_context);
        slices.par_iter_mut().for_each(|slice| {
            slice.slice_perimeters_into_chains(settings.number_of_perimeters as usize);
        });
        Ok(())
    }
}

pub struct BridgingPass;

impl SlicePass for BridgingPass {
    fn pass(
        slices: &mut Vec<Slice>,
        _settings: &Settings,
        state_context: &mut StateContext,
    ) -> Result<(), SlicerErrors> {
        state_update("Generating Moves: Bridging", state_context);

        let mut slice = slices.as_mut_slice();

        for _ in 1..slice.len() {
            let (first, second) = slice.split_at_mut(1);
            second[0].fill_solid_bridge_area(&first[0].main_polygon);
            slice = second;
        }

        Ok(())
    }
}
pub struct TopLayerPass;

impl SlicePass for TopLayerPass {
    fn pass(
        slices: &mut Vec<Slice>,
        _settings: &Settings,
        state_context: &mut StateContext,
    ) -> Result<(), SlicerErrors> {
        state_update("Generating Moves: Top Layer", state_context);

        let mut slice = slices.as_mut_slice();

        for q in 1..slice.len() {
            let (first, second) = slice.split_at_mut(1);
            first[0].fill_solid_top_layer(&second[0].main_polygon, q - 1);
            slice = second;
        }

        Ok(())
    }
}

pub struct TopAndBottomLayersPass;

impl SlicePass for TopAndBottomLayersPass {
    fn pass(
        slices: &mut Vec<Slice>,
        settings: &Settings,
        state_context: &mut StateContext,
    ) -> Result<(), SlicerErrors> {
        let top_layers = settings.top_layers;
        let bottom_layers = settings.bottom_layers;

        state_update("Generating Moves: Above and below support", state_context);

        let intersections: Vec<Option<MultiPolygon>> = slices
            .par_iter()
            .enumerate()
            .map(|(q, _slice)| {
                if (bottom_layers..slices.len() - top_layers).contains(&q) {
                    //calculate the intersection of the bottom_layers amount of layers below
                    let below = if bottom_layers != 0 {
                        Some(
                            slices[(q - bottom_layers + 1)..q].iter().fold(
                                slices
                                    .get(q - bottom_layers)
                                    .expect("Bounds Checked above")
                                    .main_polygon
                                    .clone(),
                                |a, b| a.intersection_with(&b.main_polygon),
                            ),
                        )
                    } else {
                        None
                    };
                    //calculate the intersection of the top_layers amount of layers above
                    let above = if top_layers != 0 {
                        Some(
                            slices[(q + 1)..=(q + top_layers)]
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

                    //merge top and bottom if Nessicary
                    match (above, below) {
                        (None, None) => {
                            //return empty multipolygon
                            // as a None value would be filled completely
                            Some(MultiPolygon::new(Vec::new()))
                        }
                        (None, Some(poly)) | (Some(poly), None) => Some(poly),
                        (Some(polya), Some(polyb)) => Some(polya.intersection_with(&polyb)),
                    }
                } else {
                    None
                }
            })
            .collect();

        slices
            .par_iter_mut()
            .zip(intersections)
            .enumerate()
            .for_each(|(layer, (slice, option_poly))| {
                if let Some(poly) = option_poly {
                    // fill the areas the are not part of the union of above and below layers
                    slice.fill_solid_subtracted_area(&poly, layer);
                } else {
                    //Completely fill all areas at top and bottom
                    slice.fill_remaining_area(true, layer);
                }
            });

        Ok(())
    }
}

pub struct SupportPass;

impl SlicePass for SupportPass {
    fn pass(
        slices: &mut Vec<Slice>,
        settings: &Settings,
        _state_context: &mut StateContext,
    ) -> Result<(), SlicerErrors> {
        if let Some(support) = &settings.support {
            for slice in slices.iter_mut() {
                slice.fill_support_polygons(support);
            }
        }
        Ok(())
    }
}

pub struct FillAreaPass;

impl SlicePass for FillAreaPass {
    fn pass(
        slices: &mut Vec<Slice>,
        _settings: &Settings,
        state_context: &mut StateContext,
    ) -> Result<(), SlicerErrors> {
        state_update("Generating Moves: Fill Areas", state_context);

        // Fill all remaining areas
        slices
            .par_iter_mut()
            .enumerate()
            .for_each(|(layer_num, slice)| {
                slice.fill_remaining_area(false, layer_num);
            });
        Ok(())
    }
}
pub struct LightningFillPass;

impl SlicePass for LightningFillPass {
    fn pass(
        slices: &mut Vec<Slice>,
        settings: &Settings,
        state_context: &mut StateContext,
    ) -> Result<(), SlicerErrors> {
        if settings.partial_infill_type == PartialInfillTypes::Lightning {
            state_update("Generating Moves: Lightning Infill", state_context);

            lightning_infill(slices);
        }
        Ok(())
    }
}

pub struct OrderPass;

impl SlicePass for OrderPass {
    fn pass(
        slices: &mut Vec<Slice>,
        _settings: &Settings,
        state_context: &mut StateContext,
    ) -> Result<(), SlicerErrors> {
        state_update("Generating Moves: Order Chains", state_context);

        // Fill all remaining areas
        slices.par_iter_mut().for_each(|slice| {
            slice.order_chains();
        });
        Ok(())
    }
}
