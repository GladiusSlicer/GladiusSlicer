use crate::settings::SupportSettings;
use crate::{MoveType, PolygonOperations, Slice};
use crate::plotter::{linear_fill_polygon, partial_linear_fill_polygon, support_linear_fill_polygon};

impl Slice{
    pub fn add_support_polygons(&mut self, slice_above: &Slice, support_settings: &SupportSettings){
        let distance_between_layers =  slice_above.get_height() - self.get_height() ;
        let max_overhang_distance = distance_between_layers * support_settings.max_overhang_angle.to_radians().tan();

        let current_polygon_support_area = self.main_polygon.offset_from(max_overhang_distance);
        let unsupported_above_area =  slice_above.main_polygon.difference_with(&current_polygon_support_area);

        if !unsupported_above_area.0.is_empty(){
            self.support_interface = Some(unsupported_above_area);
        }


        if let Some(above_support_interface) = &slice_above.support_interface {
            let above_support_interface_large = above_support_interface.offset_from(max_overhang_distance).difference_with(&self.main_polygon.offset_from(0.2));
            if let Some(above_support_tower) = &slice_above.support_tower {
                self.support_tower = Some(above_support_tower.union_with(&above_support_interface_large));
            }
            else{
                 self.support_tower = Some(above_support_interface_large);
            }
        }
        else{
            if let Some(above_support_tower) = &slice_above.support_tower {
                self.support_tower = Some(above_support_tower.clone());
            }
        }
    }

    pub fn fill_support_polygons(&mut self,support_settings: &SupportSettings){

        let layer_settings = &self.layer_settings;
       /* if let Some(tower_polygon) = &self.support_interface{

            self.fixed_chains.extend(
                tower_polygon
                    .iter()
                    .map(|poly| {
                        linear_fill_polygon(poly,layer_settings,MoveType::Support,0.0).into_iter()
                    })
                    .flatten()
            );
        }*/

        if let Some(tower_polygon) = &self.support_tower{

            self.fixed_chains.extend(
                tower_polygon
                    .iter()
                    .map(|poly| {
                        support_linear_fill_polygon(poly,layer_settings,MoveType::Support,support_settings.support_spacing,90.0,0.0).into_iter()
                    })
                    .flatten()
            );
        }


    }
}