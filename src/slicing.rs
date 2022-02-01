use crate::*;

pub fn slice(towers: &[TriangleTower], settings: &Settings) -> Result<Vec<Object>, SlicerErrors> {
    towers
        .iter()
        .map(|tower| {
            let mut tower_iter = TriangleTowerIterator::new(tower);

            let mut layer = 0.0;

            let mut first_layer = true;

            let slices: Vec<_> = std::iter::repeat(())
                .enumerate()
                .map(|(layer_count, _)| {
                    //Advance to the correct height
                    let layer_height = settings.get_layer_settings(layer_count, layer).layer_height;

                    let bottom_height = layer;
                    layer += layer_height / 2.0;
                    tower_iter.advance_to_height(layer)?;
                    layer += layer_height / 2.0;

                    let top_height = layer;

                    first_layer = false;

                    //Get the ordered lists of points
                    Ok((bottom_height, top_height, tower_iter.get_points()))
                })
                .take_while(|r| {
                    if let Ok((_, _, layer_loops)) = r {
                        !layer_loops.is_empty()
                    } else {
                        true
                    }
                })
                .enumerate()
                .map(|(count, r)| {
                    match r {
                        Ok((bot, top, layer_loops)) => {
                            //Add this slice to the
                            let slice = Slice::from_multiple_point_loop(
                                layer_loops
                                    .iter()
                                    .map(|verts| {
                                        verts
                                            .iter()
                                            .map(|v| Coordinate { x: v.x, y: v.y })
                                            .collect::<Vec<Coordinate<f64>>>()
                                    })
                                    .collect(),
                                bot,
                                top,
                                count,
                                settings,
                            );
                            slice
                        }
                        Err(e) => Err(e),
                    }
                })
                .try_collect()?;

            Ok(Object { layers: slices })
        })
        .try_collect()
}
