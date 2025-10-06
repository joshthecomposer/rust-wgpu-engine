use nalgebra::Vector3;
use rapier3d::prelude::*;

use crate::entity_manager::EntityManager;

pub struct PhysicsState {
    pub pipeline: PhysicsPipeline,
    pub gravity: Vector3<f32>,
    pub integration_parameters: IntegrationParameters,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhaseMultiSap,
    pub narrow_phase: NarrowPhase,
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: Option<QueryPipeline>,
    pub physics_hooks: (),
    pub event_handler: (),

    // accumulator stuff
    pub accumulator: f32,
    pub fixed_dt: f32, 
}

impl PhysicsState {
    pub fn new() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            integration_parameters: IntegrationParameters::default(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhaseMultiSap::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            gravity: vector![0.0, -9.81, 0.0],
            ccd_solver: CCDSolver::new(),
            query_pipeline: Some(QueryPipeline::new()),
            physics_hooks: (),
            event_handler: (),
            
            accumulator: 0.0,
            fixed_dt: 1.0 / 60.0,
            
        }
    }

    pub fn update(&mut self, dt: f32, em: &mut EntityManager) {
        
        self.accumulator += dt;
        let will_step = self.accumulator >= self.fixed_dt;

        if will_step {
            // roll snapshots once per frame
            for entry in em.transforms.iter() {
                em.prev_transforms.insert(entry.key(), entry.value().clone());
            }
        }

        while self.accumulator >= self.fixed_dt {
            self.integration_parameters.dt = self.fixed_dt;

            self.pipeline.step(
                &self.gravity,
                &self.integration_parameters,
                &mut self.island_manager,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.rigid_body_set,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                &mut self.ccd_solver,
                self.query_pipeline.as_mut(),
                &self.physics_hooks,
                &self.event_handler,
            );

            self.accumulator -= self.fixed_dt;
        }

        if will_step {
            for entry in em.physics_handles.iter() {
                if let (Some(rb), Some(t)) = (self.rigid_body_set.get(entry.value().rigid_body), em.transforms.get_mut(entry.key())) {
                    let iso = rb.position();
                    t.position = glam::Vec3::from_slice(iso.translation.vector.as_slice());
                    t.rotation = glam::Quat::from_array(iso.rotation.coords.as_slice().try_into().unwrap());
                }
            }
        }

        if let Some(qp) = self.query_pipeline.as_mut() {
            qp.update(&self.collider_set);
        }
    }

    pub fn interp_alpha(&self) -> f32 {
        (self.accumulator / self.fixed_dt).clamp(0.0, 1.0)
    }
}
