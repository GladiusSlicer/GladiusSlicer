use geo::algorithm::euclidean_length::EuclideanLength;
use geo::euclidean_distance::EuclideanDistance;
use geo::{Coordinate, Line};
use gladius_shared::settings::Settings;
use gladius_shared::types::{Command, StateChange};
use itertools::Itertools;

pub fn unary_optimizer(cmds: &mut Vec<Command>) {
    cmds.retain(|cmd| match cmd {
        Command::MoveTo { .. } => true,
        Command::MoveAndExtrude { start, end, .. } => start != end,
        Command::LayerChange { .. } => true,
        Command::ChangeObject { .. } => true,
        Command::SetState { new_state } => {
            !(new_state.acceleration.is_none()
                && new_state.movement_speed.is_none()
                && new_state.fan_speed.is_none()
                && new_state.retract.is_none()
                && new_state.extruder_temp.is_none()
                && new_state.bed_temp.is_none())
        }
        Command::Delay { msec } => *msec != 0,
        Command::Arc { start, end, .. } => start != end,
        Command::NoAction => false,
    });
}

pub fn binary_optimizer(cmds: &mut Vec<Command>, settings: &Settings) {
    let mut current_pos = Coordinate::zero();

    *cmds = cmds
        .drain(..)
        .coalesce(move |first, second| {
            match (first.clone(), second.clone()) {
                (
                    Command::MoveAndExtrude {
                        start: f_start,
                        end: f_end,
                        thickness: f_thick,
                        width: f_width,
                    },
                    Command::MoveAndExtrude {
                        start: s_start,
                        end: s_end,
                        thickness: s_thick,
                        width: s_width,
                    },
                ) => {
                    current_pos = s_end;

                    if f_end == s_start && s_width == f_width && s_thick == f_thick {
                        let det = (((f_start.x - s_start.x) * (s_start.y - s_end.y))
                            - ((f_start.y - s_start.y) * (s_start.x - s_end.x)))
                            .abs();

                        if det < 0.00001 {
                            //Colinear

                            return Ok(Command::MoveAndExtrude {
                                start: f_start,
                                end: s_end,
                                thickness: f_thick,
                                width: s_width,
                            });
                        }
                    }
                }
                (Command::MoveTo { .. }, Command::MoveTo { end: s_end }) => {
                    current_pos = s_end;
                    return Ok(Command::MoveTo { end: s_end });
                }

                (
                    Command::SetState { new_state: f_state },
                    Command::SetState { new_state: s_state },
                ) => {
                    return Ok(Command::SetState {
                        new_state: f_state.combine(&s_state),
                    });
                }
                (
                    Command::SetState {
                        new_state: mut f_state,
                    },
                    Command::MoveTo { end },
                ) => {
                    if f_state.retract == Some(true)
                        && Line::new(current_pos, end).euclidean_length()
                            < settings.minimum_retract_distance
                    {
                        current_pos = end;

                        //remove retract command
                        f_state.retract = None;

                        return Err((
                            Command::SetState { new_state: f_state },
                            Command::MoveTo { end },
                        ));
                    } else {
                        current_pos = end;
                    }
                }
                (
                    _,
                    Command::MoveAndExtrude {
                        start: _s_start,
                        end: s_end,
                        ..
                    },
                ) => {
                    current_pos = s_end;
                }
                (_, Command::MoveTo { end: s_end }) => {
                    current_pos = s_end;
                }
                (_, _) => {}
            }

            Err((first, second))
        })
        .collect();
}

pub fn state_optomizer(cmds: &mut Vec<Command>) {
    let mut current_state = StateChange::default();

    for cmd_ptr in cmds {
        if let Command::SetState { new_state } = cmd_ptr {
            *new_state = current_state.state_diff(new_state);
        }
    }
}

/*
pub fn arc_optomizer(cmds: &mut Vec<Command> ){
    let mut ranges = vec![];

    for (b,group) in &cmds
        .iter()
        .enumerate()
        .group_by(|cmd| {
            if let Command::MoveAndExtrude { .. } = cmd.1 { true } else { false }
        })
    {
        if b{

            let mut current_center = (0.0,0.0);
            let mut current_radius = 0.0;
            let mut current_chain = 0;

            let mut last_pos =0;


            for (pos,center,radius) in group

                //commands -> lines
                .map(|(pos,cmd)|{
                    if let Command::MoveAndExtrude {start,end} = cmd{
                        (pos,(start,end))
                    }
                    else{
                        unreachable!()
                    }
                })
                //lines -> bisector
                .tuple_windows::<((usize,(&Coordinate<f64>,&Coordinate<f64>)),(usize,(&Coordinate<f64>,&Coordinate<f64>)))>()
                .map(|((pos,l1),(_,l2))| {
                   //println!("({},{}) ({},{}) ", l1.0.x,l1.0.y,l1.1.x,l1.1.y );
                    (pos,line_bisector(l1.0,l1.1,l2.1))
                })
                //bisector -> center, radius
                .tuple_windows::<((usize,(Coordinate<f64>,Coordinate<f64>)),(usize,(Coordinate<f64>,Coordinate<f64>)))>()
                .filter_map(|((pos,(p1,n1)),(_,(p2,n2)))| {

                    //println!("({:?},{:?}) ",p1,n1 );

                    ray_ray_intersection(&p1,&n1,&p2,&n2).map(|center| (pos,center.x_y(), center.euclidean_distance(&p1)))

                }){

                last_pos = pos;

                //println!("{} ({},{}) ", radius,center.0,center.1);
                if (radius -current_radius).abs() < 1.1{
                    if (center.0 -current_center.0).abs() < 1.1 {
                        if (center.1 -current_center.1).abs() < 1.1 {
                            current_chain += 1;
                            continue;
                        }
                    }
                }

                if current_chain >5{
                    ranges.push((center,(pos- current_chain..pos)));

                    //println!("arc found {}..{}", pos- current_chain , pos);
                }


                current_center = center;
                current_radius = radius;
                current_chain = 1;




            }

            if current_chain >5{
                println!("{}..{}", current_chain , last_pos);
                ranges.push((current_center,((last_pos +2)- current_chain..last_pos)));


                //println!("arc found {}..{}", last_pos- current_chain , last_pos);
            }


        }
    }

    for (center,mut range) in ranges{
        let start = if let Command::MoveAndExtrude {start,..} = cmds[range.start]{
            start
        }else{
            unreachable!()
        };
        let end = if let Command::MoveAndExtrude {end,..} = cmds[range.end]{
            end
        }else{
            unreachable!()
        };



        for i in range.clone(){
            cmds[i] = Command::NoAction;
        }

        cmds[range.start] = Command::Arc {start,end,clockwise: true,center: Coordinate{x: center.0, y: center.1}};

    }

}
*/

fn line_bisector(
    p0: &Coordinate<f64>,
    p1: &Coordinate<f64>,
    p2: &Coordinate<f64>,
) -> (Coordinate<f64>, Coordinate<f64>) {
    let ray_start = *p1;

    let l1_len = p0.euclidean_distance(p1);
    let l2_len = p1.euclidean_distance(p2);

    let l1_unit = (*p1 - *p0) / -l1_len;
    let l2_unit = (*p1 - *p2) / -l2_len;

    let dir = l1_unit + l2_unit;

    (ray_start, dir)
}

fn ray_ray_intersection(
    s0: &Coordinate<f64>,
    d0: &Coordinate<f64>,
    s1: &Coordinate<f64>,
    d1: &Coordinate<f64>,
) -> Option<Coordinate<f64>> {
    let dx = s1.x - s0.x;
    let dy = s1.y - s0.y;

    let det = d1.x * d0.y - d1.y * d0.x;
    let u = (dy * d1.x - dx * d1.y) / det;
    let v = (dy * d0.x - dx * d0.y) / det;
    if (u > 0.0) && (v > 0.0) {
        //println!("({},{}) ", p1.x,p1.y, );
        let p1_end = *s0 + *d0; // another point in line p1->n1
        let p2_end = *s1 + *d1; // another point in line p2->n2

        let m1 = (p1_end.y - s0.y) / (p1_end.x - s0.x); // slope of line p1->n1
        let m2 = (p2_end.y - s1.y) / (p2_end.x - s1.x); // slope of line p2->n2

        let b1 = s0.y - m1 * s0.x; // y-intercept of line p1->n1
        let b2 = s1.y - m2 * s1.x; // y-intercept of line p2->n2

        let px = (b2 - b1) / (m1 - m2); // collision x
        let py = m1 * px + b1; // collision y

        Some(Coordinate { x: px, y: py })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_line_bisector() {
        let (center, dir) = line_bisector(
            &Coordinate { x: 0.0, y: 0.0 },
            &Coordinate { x: 1.0, y: 1.0 },
            &Coordinate { x: 2.0, y: 0.0 },
        );

        assert_eq!(center, Coordinate { x: 1.0, y: 1.0 });
        assert_eq!(dir.x, 0.0);
        assert!(dir.y < 0.0);

        let (center, dir) = line_bisector(
            &Coordinate { x: 2.0, y: 0.0 },
            &Coordinate { x: 1.0, y: 1.0 },
            &Coordinate { x: 0.0, y: 0.0 },
        );

        assert_eq!(center, Coordinate { x: 1.0, y: 1.0 });
        assert_eq!(dir.x, 0.0);
        assert!(dir.y < 0.0);

        let (center, dir) = line_bisector(
            &Coordinate { x: 0.0, y: 0.0 },
            &Coordinate { x: 1.0, y: 1.0 },
            &Coordinate { x: -2.0, y: 4.0 },
        );

        assert_eq!(center, Coordinate { x: 1.0, y: 1.0 });
        assert_eq!(dir.y, 0.0);
        assert!(dir.x < 0.0);

        let (center, dir) = line_bisector(
            &Coordinate { x: 0.0, y: 0.0 },
            &Coordinate { x: 1.0, y: 0.0 },
            &Coordinate { x: 1.0, y: 1.0 },
        );

        assert_eq!(center, Coordinate { x: 1.0, y: 0.0 });
        assert_eq!(dir.y, -dir.x);
    }

    #[test]
    fn basic_ray_ray() {
        let center = ray_ray_intersection(
            &Coordinate { x: 0.0, y: 0.0 },
            &Coordinate { x: 1.0, y: 1.0 },
            &Coordinate { x: 2.0, y: 0.0 },
            &Coordinate { x: -1.0, y: 1.0 },
        );
        assert_eq!(center, Some(Coordinate { x: 1.0, y: 1.0 }));

        let center = ray_ray_intersection(
            &Coordinate { x: 0.0, y: 3.0 },
            &Coordinate { x: 5.0, y: 1.0 },
            &Coordinate { x: 2.0, y: 0.0 },
            &Coordinate { x: 3.0, y: 4.0 },
        );
        assert_eq!(center, Some(Coordinate { x: 5.0, y: 4.0 }));

        let center = ray_ray_intersection(
            &Coordinate { x: 1.0, y: 3.0 },
            &Coordinate { x: 0.10, y: -0.20 },
            &Coordinate { x: 0.0, y: -2.0 },
            &Coordinate { x: 2.0, y: 3.0 },
        );
        assert_eq!(center, Some(Coordinate { x: 2.0, y: 1.0 }));

        let center = ray_ray_intersection(
            &Coordinate {
                x: 1112.4,
                y: 35345.0,
            },
            &Coordinate {
                x: -0.11124,
                y: -3.53450,
            },
            &Coordinate {
                x: 546456.1,
                y: 544456.1,
            },
            &Coordinate {
                x: -0.5464561,
                y: -0.5444561,
            },
        );
        //assert_eq!(center, Some(Coordinate{x: 0.0,y:0.0}));
    }

    #[test]
    fn arc_optomizer_test() {
        let mut commands = (0..600)
            .into_iter()
            .map(|a| {
                let r = a as f64 / 100.0;
                let x = r.cos();
                let y = r.sin();
                Coordinate { x, y }
            })
            .tuple_windows::<(Coordinate<f64>, Coordinate<f64>)>()
            .map(|(start, end)| Command::MoveAndExtrude { start, end })
            .collect::<Vec<Command>>();

        arc_optomizer(&mut commands);
        unary_optimizer(&mut commands);

        assert_eq!(commands, vec![])
    }
}
