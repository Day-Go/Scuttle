use rand::{Rng, thread_rng};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_prototype_lyon::prelude::*;
use rayon::prelude::*;


pub struct Density {
    pub value: f32,
}

#[derive(Component)]
struct Particle;

fn smoothing_kernel(r: f32, d: f32) -> f32 {
    let volume: f32 = 78539816.0;
    let r_squared = r.powi(2);
    let d_squared = d.powi(2);
    let squared_distance: f32 = r_squared - d_squared;

    let value = if squared_distance > 0.0 { 
        // println!("Volume: {}", volume);
        // println!("Radius: {}, Dist: {}", r_squared, d_squared);
        // println!("Squared Distance: {}\n", squared_distance);
        squared_distance
    } else { 
        return 0.0; 
    };
    let normalized_value = value.powi(3) / volume;

    // Normalize the output
    let max_value = 4.0 / (std::f32::consts::PI * r.powi(2));
    normalized_value * max_value
}


#[derive(Component)]
struct Cell {
    pub density: f32,
    pub pressure: f32,
}

impl Cell {
    pub fn update_density(&mut self, density: f32) {
        self.density += density;
    }

    pub fn reset_density(&mut self) {
        self.density = 0.0;
    }

    pub fn update_cell_colour(&self, fill: &mut Fill) {
        
        let red = self.density; 
        let blue = 1.0 - self.density; 
        let green = 1.0 - (red - blue).abs();
    
        let colour = Color::rgb(red, green, blue);
        fill.color = colour;
    }
}


fn main() {
    let window_width = 1320.0;
    let window_height = 780.0; 

    let cell_size = 20.0;
    let particle_radius: f32 = 4.0;
    let n_particles: usize = 100;
    let particle_spacing: f32 = 50.0;

    App::new()
        .insert_resource(Msaa::Off)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "I am a window!".into(),
                    resolution: (window_width, window_height).into(),
                    ..default()
                }),
                ..default()
            }),
        ))
        .add_plugins(ShapePlugin)
        .add_plugins(RapierPhysicsPlugin::<()>::default())
        .add_systems(Startup, setup_graphics)
        .add_systems(Startup, move |commands: Commands| 
            setup_cells(commands, &cell_size, &window_width, &window_height))
        .add_systems(Startup, move |commands: Commands|
            setup_bounding_box(commands, &window_width, &window_height))
        .add_systems(Startup, move |commands: Commands| 
            setup_particles(commands, &particle_radius, &n_particles, 
                            &particle_spacing))
        .add_systems(Update, 
            (calculate_density)
        )
        .add_systems(PostUpdate, |mut query: Query<&mut Cell>| {
            for mut cell in query.iter_mut() {
                cell.reset_density();
            }
        })
        .run();
}

fn setup_graphics(mut commands: Commands) 
{
    commands.spawn(Camera2dBundle::default());
}

fn setup_bounding_box(mut commands: Commands, width: &f32, height: &f32) {
    // Create bounding box
    commands.spawn(RigidBody::Fixed)
        .insert(Collider::cuboid(width / 2.0, 10.0))
        .insert(TransformBundle::from(
            Transform::from_xyz(0.0, -height / 2.0 - 10.0, 0.0)
        ))
        .insert(Restitution::new(1.0));

    commands.spawn(RigidBody::Fixed)
        .insert(Collider::cuboid(width / 2.0, 10.0))
        .insert(TransformBundle::from(
            Transform::from_xyz(0.0, height / 2.0 + 10.0, 0.0)
        ))
        .insert(Restitution::new(1.0));

    commands.spawn(RigidBody::Fixed)
        .insert(Collider::cuboid(10.0, height / 2.0))
        .insert(TransformBundle::from(
            Transform::from_xyz(-width / 2.0 - 10.0, 0.0, 0.0)
        ))
        .insert(Restitution::new(1.0));

    commands.spawn(RigidBody::Fixed)
        .insert(Collider::cuboid(10.0, height / 2.0))
        .insert(TransformBundle::from(
            Transform::from_xyz(width / 2.0 + 10.0, 0.0, 0.0)
        ))
        .insert(Restitution::new(1.0));
}

fn setup_cells(mut commands: Commands, cell_size: &f32, width: &f32, height: &f32) {
    let cell_size = *cell_size;
    let cell_spacing = 0.0;

    // Calculate the number of cells that can fit in the width and height
    let cells_x = (width / (cell_size + cell_spacing)).floor() as i32;
    let cells_y = (height / (cell_size + cell_spacing)).floor() as i32;

    // Loop to create the grid of cells
    for x in (-cells_x / 2.0 as i32)..(cells_x / 2.0 as i32) {
        for y in (-cells_y / 2.0 as i32)..((cells_y / 2.0 as i32) + 1) {
            // Calculate the position for each cell
            let pos_x = x as f32 * cell_size + cell_size / 2.0;
            let pos_y = y as f32 * cell_size;

            let shape = shapes::Rectangle {
                extents: Vec2::new(cell_size, cell_size),
                ..shapes::Rectangle::default()
            };

            // Create a cell and set its position
            commands
                .spawn((
                    ShapeBundle {
                        path: GeometryBuilder::build_as(&shape),
                        ..default()
                    },
                    Fill::color(Color::BLUE),
                    Stroke::new(Color::BLACK, 1.0),
                ))
                .insert(Collider::cuboid(cell_size / 2.0, cell_size / 2.0))
                .insert(Sensor)
                .insert(TransformBundle::from(
                    Transform::from_xyz(pos_x, pos_y, -1.0)
                ))
                .insert(Cell {
                    density: 0.0,
                    pressure: 0.0,
                });
        }
    }
}

// Goal 1: Get two particles to repel from each other
fn setup_particles(mut commands: Commands, 
                    particle_radius: &f32, 
                    n_particles: &usize,
                    particle_spacing: &f32) {
    let particles_per_row: usize = (*n_particles as f64).sqrt() as usize;
    let particles_per_column: usize = (n_particles - 1) / particles_per_row + 1;
    let spacing: f32 = (particle_radius * 2.0) + particle_spacing;

    let g1 = Group::from_bits(0b1000).unwrap();
    let g2 = Group::from_bits(0b0111).unwrap();
    for i in 0..*n_particles {
        let x = (i % particles_per_row) as f32 * spacing - (particles_per_row as f32 * spacing) / 2.0;
        let y = (i / particles_per_row) as f32 * spacing - (particles_per_column as f32 * spacing) / 2.0;

        let shape = shapes::Circle {
            radius: *particle_radius,
            center: Vec2::ZERO,
        };
    
        commands
            .spawn((
                ShapeBundle {
                    path: GeometryBuilder::build_as(&shape),
                    ..default()
                },
                Fill::color(Color::CYAN),
                Stroke::new(Color::BLACK, 1.0),
            ))
            .insert(Particle)
            .insert(RigidBody::Dynamic)
            .insert(Collider::ball(*particle_radius))
            .insert(TransformBundle::from(
                Transform::from_xyz(x, y, 0.0)
            ))
            .insert(CollisionGroups::new(g1, g2))
            .insert(GravityScale(0.0))
            .insert(Velocity::linear(Vec2::new(100.0, 10.0)))
            .insert(ExternalForce {
                force: Vec2::ZERO.into(),
                torque: 0.0, 
            })
            ;
    }
}

fn calculate_density(pos_query: Query<(&Transform, With<Particle>)>,
                     mut cell_query: Query<(Entity, &Transform, &mut Cell, &mut Fill)>) {
    
    let positions: Vec<&Transform> = pos_query
        .iter()
        .map(|(transform, _)| transform)
        .collect();

    for p1 in positions {
        let influence_radius: f32 = 75.0;

        let overlapping_cells: Vec<(_, f32)> = cell_query.iter_mut()
            .filter_map(|(entity, p2, cell, fill)| {
                let distance = (p1.translation - p2.translation).length();
                if distance <= influence_radius {
                    Some(((entity, p2, cell, fill), distance))
                } else {
                    None
                }
            })
            .collect();

        for ((_, _, mut cell, mut fill), distance) in overlapping_cells {
            let density = smoothing_kernel(influence_radius, distance);
            cell.update_density(density);
            cell.update_cell_colour(&mut fill);
        }
    }
}

// fn calculate_density(pos_query: Query<(&Transform, With<Particle>)>,
//                      mut cell_query: Query<(Entity, &Transform, &mut Cell, &mut Fill)>) {
    
//     let positions: Vec<&Transform> = pos_query
//         .iter()
//         .map(|(transform, _)| transform)
//         .collect();

//     positions.par_iter().for_each(|p1| {
//         let influence_radius: f32 = 75.0;

//         cell_query.par_iter_mut()
//             .filter_map(|(entity, p2, cell, fill)| {
//                 let distance = (p1.translation - p2.translation).length();
//                 if distance <= influence_radius {
//                     Some(((entity, p2, cell, fill), distance))
//                 } else {
//                     None
//                 }
//             })
//             .for_each(|((_, _, mut cell, mut fill), distance)| {
//                 let density = smoothing_kernel(influence_radius, distance);
//                 cell.update_density(density);
//                 cell.update_cell_colour(&mut fill);
//             });
//     });
// }



// let mut rng = thread_rng();
// let mut density: f32 = 0.0;
// let mut neighbours: Vec<&Transform> = Vec::new();
// let mut neighbour_densities: Vec<f32> = Vec::new();
// let mut neighbour_distances: Vec<f32> = Vec::new();

// // Get all the particles in the scene
// for (other_transform, _) in query.iter_mut() {
//     // Calculate the distance between the two particles
//     let distance = transform.translation.distance(other_transform.translation);
//     // If the distance is less than the smoothing radius, add it to the neighbours list
//     if distance < 20.0 {
//         neighbours.push(other_transform);
//         neighbour_distances.push(distance);
//     }
// }

// // Calculate the density of the particle
// for neighbour in neighbours {
//     let distance = transform.translation.distance(neighbour.translation);
//     let density = smoothing_kernel(20.0, distance);
//     neighbour_densities.push(density);
// }

// for density in neighbour_densities {
//     density += density;
// }

// // Update the particle's density
// density = density / neighbours.len() as f32;
// println!("Density: {}", density);
// Cell::update_cell_colour(&density, &mut query.get_mut().unwrap().1);