// #[derive(Component)]
// pub enum Charge {
//     Positive,
//     Negative,
//     Neutral
// }

// #[derive(Bundle)]
// pub struct ParticleState  {
//     pub charge: Charge

// }

// impl ParticleState {
//     pub fn new(charge: Charge) -> Self {
//         ParticleState {
//             charge
//         }
//     }
// }

// .insert(ParticleState::new(Charge::Negative))


use rand::{Rng, thread_rng};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_prototype_lyon::prelude::*;
use rayon::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::ecs::system::SystemParam;


pub struct Density {
    pub value: f32,
}

#[derive(Component)]
struct Particle;

fn smoothing_kernel(r: f32, d: f32) -> f32 {
    let volume: f32 = 78539816.0;
    let r_squared: f32 = r.powf(2.0);
    let d_squared: f32 = d.powf(2.0);
    let squared_distance: f32 = r_squared - d_squared;

    let value = if squared_distance > 0.0 { 
        // println!("Volume: {}", volume);
        // println!("Radius: {}, Dist: {}", r_squared, d_squared);
        // println!("Squared Distance: {}\n", squared_distance);
        squared_distance
    } else { 
        0.0 
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
    pub fn update(&mut self, center: &Transform, particles: &Vec<(&Particle, &Transform)>) {
        let mut density: f32 = 0.0;
        const mass: f32 = 1.0;

        // Use a parallel iterator to process particles
        density = particles.par_iter()
            .map(|(_, transform)| {
                let vector = center.translation - transform.translation;
                let distance = vector.length();
                let influence = smoothing_kernel(75.0, distance);
                mass * influence
            })
            .sum();

        self.density = density;
    }
}


fn main() {
    let window_width = 1320.0;
    let window_height = 780.0; 

    let cell_size = 20.0;
    let particle_radius: f32 = 4.0;
    let n_particles: usize = 160;
    let particle_spacing: f32 = 4.0;

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
            setup_particles(commands, &particle_radius, &n_particles, &particle_spacing)) 
        .add_systems(Update, 
            (repulsion_system, update_cell_density)
        )
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
            .insert(Collider::ball(*particle_radius))
            .insert(CollisionGroups::new(g1, g2))
            .insert(GravityScale(0.0))
            .insert(ExternalForce {
                force: Vec2::ZERO.into(),
                torque: 0.0, 
            })
            ;
    }
}

fn repulsion_system(mut query: Query<(&mut ExternalForce, &Transform)>,) {
    // Collecting entities and their associated data
    let particles: Vec<_> = query.iter_mut().collect();
    let particle_count = particles.len();

    // Preparing a vector to store calculated forces
    let mut forces = vec![Vec3::ZERO; particle_count];

    // Calculating forces
    for i in 0..particle_count {
        
        for j in 0..particle_count {
            if i != j {
                let (_, transform1) = &particles[i];
                let (_, transform2) = &particles[j];
                
                let direction = transform1.translation - transform2.translation;
                let distance = direction.length();
                let force = (direction.normalize() / distance.powi(2)) * 10000000.0;
                forces[i] += force;
            }
        }
    }

    // Applying the forces
    for (i, (mut force, _)) in particles.into_iter().enumerate() {
        force.force.x = forces[i].x;
        force.force.y = forces[i].y;
    }
}

fn update_cell_density(mut cell_query: Query<(&mut Cell, &Transform, &mut Fill)>,
                       particle_query: Query<(&Particle, &Transform)>) {
    
    for (mut cell, cell_transform, mut fill) in cell_query.iter_mut() {
        // Get the position of the cell
        let cell_position = cell_transform.translation;

        // Filter the particles based on the distance to the cell
        let nearby_particles: Vec<_> = particle_query.iter()
        .filter(|(_, particle_transform)| {
            let distance_squared = (cell_position - particle_transform.translation).length_squared();
            distance_squared <= (75.0 * 75.0)
        })
        .collect();

        cell.update(&cell_transform, &nearby_particles);

        if nearby_particles.len() > 0 {
            update_cell_colour(&cell.density, &mut fill);
        }
    }
}


fn update_cell_colour(density: &f32, fill: &mut Fill) {
    let red = density; 
    let blue = 1.0 - density; 
    let green = 1.0 - (red - blue).abs();

    let colour = Color::rgb(*red, green, blue);

    fill.color = colour;
    
}
