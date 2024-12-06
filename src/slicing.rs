use crate::{
    tower::TowerVertex, Coord, Object, Settings, Slice, SlicerErrors, TriangleTower, TriangleTowerIterator
};
use rayon::{
    iter::{IntoParallelIterator, ParallelBridge, ParallelIterator},
    slice::ParallelSliceMut,
};
use std::fmt::Debug;

pub fn slice<V>(
    towers: Vec<TriangleTower<V>>,
    settings: &Settings,
) -> Result<Vec<Object>, SlicerErrors>
where V: Sync + Clone + TowerVertex + Debug {
    towers
        .into_par_iter()
        .map(|tower| {
            let mut tower_iter = TriangleTowerIterator::new(tower);

            let mut layer = 0.0;

            // loop in max layers of printer
            let slices: Result<Vec<Slice>, SlicerErrors> = (0..(settings.print_z / settings.layer_height).round() as u32)
                .map(|layer_count| {
                    // Advance to the correct height
                    let layer_height = settings.get_layer_settings(layer_count, layer).layer_height;

                    let bottom_height = layer;
                    layer += layer_height / 2.0;
                    tower_iter.advance_to_height(layer)?;
                    layer += layer_height / 2.0;

                    let top_height = layer;

                    // Get the ordered lists of points
                    Ok((bottom_height, top_height, tower_iter.get_points(),tower_iter.is_finished()))
                })
                .take_while(|r| {
                    if let Ok((_, _, _, finished)) = r {
                        !finished
                    } else {
                        true
                    }
                })
                .enumerate()
                .par_bridge()
                .map(|(count, result)| {
                    result.and_then(|(bot, top, layer_loops, _)| {
                        // Add this slice to the
                        let slice = Slice::from_multiple_point_loop(
                            layer_loops
                                .iter()
                                .map(|verts| {
                                    verts
                                        .iter()
                                        .map(|v| Coord {
                                            x: v.get_slice_x(),
                                            y: v.get_slice_y(),
                                        })
                                        .collect::<Vec<Coord<f64>>>()
                                })
                                .collect(),
                            bot,
                            top,
                            count as u32, // I doute your layer_count will go past 4,294,967,295,
                            settings,
                        );
                        slice
                    })
                })
                .collect();
            let mut s = slices?;

            // sort as parbridge isn't guaranteed to return in order
            s.par_sort_by(|a, b| {
                a.top_height
                    .partial_cmp(&b.top_height)
                    .expect("No NAN are in height")
            });

            Ok(Object { layers: s })
        })
        .collect()
}
