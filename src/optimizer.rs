use geo::{Coord, Euclidean, Length, Line, Distance};
use gladius_shared::settings::Settings;
use gladius_shared::types::{Command, RetractionType, StateChange};
use itertools::Itertools;

/// Remove [`Command`]s that don't acheve anything
pub fn unary_optimizer(cmds: &mut Vec<Command>) {
    cmds.retain(|cmd| match cmd {
        Command::MoveTo { .. } => true,
        Command::MoveAndExtrude { start, end, .. } => start != end,
        Command::LayerChange { .. } | Command::ChangeObject { .. } => true,
        Command::SetState { new_state } => {
            !(new_state.acceleration.is_none()
                && new_state.movement_speed.is_none()
                && new_state.fan_speed.is_none()
                && new_state.retract == RetractionType::NoRetract
                && new_state.extruder_temp.is_none()
                && new_state.bed_temp.is_none())
        }
        Command::Delay { msec } => *msec != 0,
        Command::Arc {
            start, end, center, ..
        } => start != end || start != center,
        Command::NoAction => false,
    });
}

pub fn binary_optimizer(cmds: &mut Vec<Command>, settings: &Settings) {
    let mut current_pos = Coord::zero();

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
                            // Colinear
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
                (Command::Delay { msec: t1 }, Command::Delay { msec: t2 }) => {
                    // merge back to back delays
                    return Ok(Command::Delay { msec: t1 + t2 });
                }
                (Command::ChangeObject { .. }, Command::ChangeObject { object }) => {
                    // skip an object change followed by another change
                    return Ok(Command::ChangeObject { object });
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
                    if f_state.retract == RetractionType::Retract
                        && Line::new(current_pos, end).length::<Euclidean>()
                            < settings.minimum_retract_distance
                    {
                        current_pos = end;

                        // remove retract command
                        f_state.retract = RetractionType::NoRetract;

                        return Err((
                            Command::SetState { new_state: f_state },
                            Command::MoveTo { end },
                        ));
                    } else if let RetractionType::MoveRetract(_) = f_state.retract {
                        if Line::new(current_pos, end).length::<Euclidean>()
                            < settings.minimum_retract_distance
                        {
                            current_pos = end;

                            // remove retract command
                            f_state.retract = RetractionType::NoRetract;

                            return Err((
                                Command::SetState { new_state: f_state },
                                Command::MoveTo { end },
                            ));
                        }
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

pub fn arc_optomizer(cmds: &mut [Command]) {
    let mut ranges = vec![];

    for (wt, group) in &cmds.iter().enumerate().chunk_by(|cmd| {
        if let Command::MoveAndExtrude {
            thickness, width, ..
        } = cmd.1 {
            Some((thickness, width))
        } else {
            None
        }
    }) {
        if let Some((thickness, width)) = wt {
            let mut current_center = (0.0, 0.0);
            let mut current_radius = 0.0;
            let mut current_chain = 0;

            let mut last_pos = 0;
            let mut group_peek = group.peekable();
            let mut start_pos = group_peek.peek().expect("validated aboive").0;

            for (pos, center, radius) in group_peek
                // commands -> lines
                .map(|(pos, cmd)| {
                    if let Command::MoveAndExtrude { start, end, .. } = cmd {
                        (pos, (start, end))
                    } else {
                        unreachable!()
                    }
                })
                // lines -> bisector
                .tuple_windows::<(
                    (usize, (&Coord<f64>, &Coord<f64>)),
                    (usize, (&Coord<f64>, &Coord<f64>)),
                )>()
                .map(|((pos, l1), (_, l2))| {
                    // todo try to avoid copy
                    (pos, line_bisector(*l1.0, *l1.1, *l2.1))
                })
                // bisector -> center, radius
                .tuple_windows::<(
                    (usize, (Coord<f64>, Coord<f64>)),
                    (usize, (Coord<f64>, Coord<f64>)),
                )>()
                .filter_map(|((pos, (p1, n1)), (_, (p2, n2)))| {
                    ray_ray_intersection(&p1, &n1, &p2, &n2)
                        .map(|center| (pos, center.x_y(), Euclidean::distance(center, p1)))
                })
            {
                last_pos = pos;

                if (radius - current_radius).abs() < 1.1
                    && (center.0 - current_center.0).abs() < 1.1
                    && (center.1 - current_center.1).abs() < 1.1
                {
                    current_chain += 1;
                    continue;
                }

                if current_chain > 5 {
                    ranges.push((center, (start_pos..=pos), *thickness, *width));
                }

                current_center = center;
                current_radius = radius;
                current_chain = 1;
                start_pos = pos;
            }

            if current_chain > 5 {
                ranges.push((
                    current_center,
                    (start_pos..=last_pos + 2),
                    *thickness,
                    *width,
                ));
            }
        }
    }

    for (center, mut range, thickness, width) in ranges {
        // todo fix
        let start = match cmds[*range.start()] {
            Command::MoveAndExtrude { start, .. } => start,
            _ => continue,
        };
        let end = match cmds[*range.end()] {
            Command::MoveAndExtrude { end, .. } => end,
            _ => continue,
        };

        for i in range.by_ref() {
            cmds[i] = Command::NoAction;
        }

        cmds[*range.start()] = Command::Arc {
            start,
            end,
            clockwise: true,
            center: Coord {
                x: center.0,
                y: center.1,
            },
            thickness,
            width,
        };
    }
}

fn line_bisector(p0: Coord<f64>, p1: Coord<f64>, p2: Coord<f64>) -> (Coord<f64>, Coord<f64>) {
    let l1_len = Euclidean::distance(p0, p1);
    let l2_len = Euclidean::distance(p1, p2);

    let l1_unit = (p1 - p0) / -l1_len;
    let l2_unit = (p1 - p2) / -l2_len;

    let dir = l1_unit + l2_unit;

    (/* ray_start */ p1, dir)
}

fn ray_ray_intersection(
    s0: &Coord<f64>,
    d0: &Coord<f64>,
    s1: &Coord<f64>,
    d1: &Coord<f64>,
) -> Option<Coord<f64>> {
    let dx = s1.x - s0.x;
    let dy = s1.y - s0.y;

    let det = d1.x * d0.y - d1.y * d0.x;
    let u = (dy * d1.x - dx * d1.y) / det;
    let v = (dy * d0.x - dx * d0.y) / det;
    if (u > 0.0) && (v > 0.0) {
        let p1_end = *s0 + *d0; // another point in line p1->n1
        let p2_end = *s1 + *d1; // another point in line p2->n2

        let m1 = (p1_end.y - s0.y) / (p1_end.x - s0.x); // slope of line p1->n1
        let m2 = (p2_end.y - s1.y) / (p2_end.x - s1.x); // slope of line p2->n2

        let b1 = s0.y - m1 * s0.x; // y-intercept of line p1->n1
        let b2 = s1.y - m2 * s1.x; // y-intercept of line p2->n2

        let px = (b2 - b1) / (m1 - m2); // collision x
        let py = m1 * px + b1; // collision y

        Some(Coord { x: px, y: py })
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
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 1.0, y: 1.0 },
            Coord { x: 2.0, y: 0.0 },
        );

        assert_eq!(center, Coord { x: 1.0, y: 1.0 });
        assert_eq!(dir.x, 0.0);
        assert!(dir.y < 0.0);

        let (center, dir) = line_bisector(
            Coord { x: 2.0, y: 0.0 },
            Coord { x: 1.0, y: 1.0 },
            Coord { x: 0.0, y: 0.0 },
        );

        assert_eq!(center, Coord { x: 1.0, y: 1.0 });
        assert_eq!(dir.x, 0.0);
        assert!(dir.y < 0.0);

        let (center, dir) = line_bisector(
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 1.0, y: 1.0 },
            Coord { x: -2.0, y: 4.0 },
        );

        assert_eq!(center, Coord { x: 1.0, y: 1.0 });
        assert!(dir.y - 0.0 < 0.000001);
        assert!(dir.x < 0.0);

        let (center, dir) = line_bisector(
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 1.0, y: 0.0 },
            Coord { x: 1.0, y: 1.0 },
        );

        assert_eq!(center, Coord { x: 1.0, y: 0.0 });
        assert_eq!(dir.y, -dir.x);
    }

    #[test]
    fn basic_ray_ray() {
        let center = ray_ray_intersection(
            &Coord { x: 0.0, y: 0.0 },
            &Coord { x: 1.0, y: 1.0 },
            &Coord { x: 2.0, y: 0.0 },
            &Coord { x: -1.0, y: 1.0 },
        );
        assert_eq!(center, Some(Coord { x: 1.0, y: 1.0 }));

        let center = ray_ray_intersection(
            &Coord { x: 0.0, y: 3.0 },
            &Coord { x: 5.0, y: 1.0 },
            &Coord { x: 2.0, y: 0.0 },
            &Coord { x: 3.0, y: 4.0 },
        );
        assert_eq!(center, Some(Coord { x: 5.0, y: 4.0 }));

        let center = ray_ray_intersection(
            &Coord { x: 1.0, y: 3.0 },
            &Coord { x: 0.10, y: -0.20 },
            &Coord { x: 0.0, y: -2.0 },
            &Coord { x: 2.0, y: 3.0 },
        );
        assert_eq!(center, Some(Coord { x: 2.0, y: 1.0 }));
    }

    #[test]
    fn arc_optomizer_test() {
        let mut commands = (0..200)
            .map(|a| {
                let r = a as f64 / 100.0;
                let x = r.cos();
                let y = r.sin();
                Coord { x, y }
            })
            .tuple_windows::<(Coord<f64>, Coord<f64>)>()
            .map(|(start, end)| Command::MoveAndExtrude {
                start,
                end,
                thickness: 0.3,
                width: 0.4,
            })
            .collect::<Vec<Command>>();

        arc_optomizer(&mut commands);
        unary_optimizer(&mut commands);

        assert_eq!(commands.len(), 1);
        if let Command::Arc {
            start,
            center,
            width,
            thickness,
            ..
        } = commands[0] {
            assert_eq!(start, Coord { x: 1.0, y: 0.0 });
            assert_eq!(center, Coord { x: 0.0, y: 0.0 });
            assert_eq!(width, 0.4);
            assert_eq!(thickness, 0.3);
        } else {
            panic!("Command should be an arc")
        }
    }

    #[test]
    fn arc_optomizer_test_adv() {
        let mut commands = vec![Command::Delay { msec: 1000 }];

        commands.extend(
            (0..200)
                .map(|a| {
                    let r = a as f64 / 100.0;
                    let x = r.cos();
                    let y = r.sin();
                    Coord { x, y }
                })
                .tuple_windows::<(Coord<f64>, Coord<f64>)>()
                .map(|(start, end)| Command::MoveAndExtrude {
                    start,
                    end,
                    thickness: 0.3,
                    width: 0.4,
                }),
        );

        commands.push(Command::Delay { msec: 1000 });

        arc_optomizer(&mut commands);
        unary_optimizer(&mut commands);

        assert_eq!(commands.len(), 3);
        if let Command::Arc {
            start,
            center,
            width,
            thickness,
            ..
        } = commands[1] {
            assert_eq!(start, Coord { x: 1.0, y: 0.0 });
            assert_eq!(center, Coord { x: 0.0, y: 0.0 });
            assert_eq!(width, 0.4);
            assert_eq!(thickness, 0.3);
        } else {
            panic!("Command should be an arc")
        }
    }
}
