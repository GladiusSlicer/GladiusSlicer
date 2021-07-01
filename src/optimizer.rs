use crate::types::{Command, StateChange};
use itertools::{Itertools};
use geo::{Line, Coordinate};
use geo::algorithm::euclidean_length::EuclideanLength;
use crate::settings::Settings;

pub fn optimize_commands(cmds: &mut Vec<Command> ,settings:& Settings) {

    let mut size  = cmds.len();

    while {


        state_optomizer(cmds);
        unary_optimizer(cmds);
        binary_optimizer(cmds,settings);

         cmds.len() != size

    }{
        size = cmds.len()
    }

}


pub fn unary_optimizer(cmds: &mut Vec<Command> ){



    cmds.retain(|cmd|{

        match cmd{
            Command::MoveTo { .. } => { true }
            Command::MoveAndExtrude { start, end } => { start != end }
            Command::LayerChange { .. } => {true }
            Command::SetState { new_state } => {
                !(new_state.ExtruderTemp.is_none() && new_state.MovementSpeed.is_none() && new_state.Retract.is_none() && new_state.ExtruderTemp.is_none() && new_state.BedTemp.is_none() )
            }
            Command::Delay { msec } => { *msec !=0}
            Command::Arc { start, end,.. } => {start != end}
            Command::NoAction => {false}
        }

    });

}

pub fn binary_optimizer(cmds: &mut Vec<Command> , settings: &Settings){

    let mut current_pos = Coordinate::zero();

    *cmds = cmds.drain(..).coalesce(move |first,second|{

        match (first.clone(),second.clone()){
            (Command::MoveAndExtrude {start: f_start,end: f_end}, Command::MoveAndExtrude {start : s_start,end:s_end}) => {
                current_pos = s_end;

                if f_end == s_start {
                    let det = (((f_start.x - s_start.x)*(s_start.y-s_end.y)) - ((f_start.y - s_start.y)*(s_start.x-s_end.x)) ).abs();

                    if det < 0.00001{
                        //Colinear

                        return Ok(Command::MoveAndExtrude { start: f_start, end: s_end });
                    }
                }
            }
            (Command::MoveTo {..}, Command::MoveTo {end:s_end}) => {
                current_pos = s_end;
                return Ok(Command::MoveTo { end: s_end });
            }

            (Command::SetState {new_state:f_state}, Command::SetState {new_state:s_state}) => {

                return Ok(Command::SetState {new_state: f_state.combine(&s_state)} );
            }
            (Command::SetState {new_state:f_state},Command::MoveTo {end}) => {
                if f_state.Retract == Some(true) && Line::new(current_pos,end).euclidean_length() < settings.minimum_retract_distance{
                    current_pos = end;
                    return Ok(Command::MoveTo {end} );
                }
                else{
                    current_pos = end;
                }
            }
            (_, Command::MoveAndExtrude {start : s_start,end:s_end}) => {
                current_pos = s_end;
            }
            (_, Command::MoveTo {end:s_end}) => {
                current_pos = s_end;
            }
            (_, _) => {}
        }

        Err((first,second))
    }).collect();

}



pub fn state_optomizer(cmds: &mut Vec<Command> ){

    let mut current_state =StateChange::default();

    for cmd_ptr in  cmds{
        if let Command::SetState {new_state} =cmd_ptr{
            *new_state = current_state.state_diff(&new_state);
        }
    };


}


