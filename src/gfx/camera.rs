//! Isometric-style camera: fixed yaw/pitch, smooth follow, 3/4 top view.

use glam::{Mat4, Vec3, Vec4};

pub struct Camera {
    pub target: Vec3,
    pub smooth: Vec3,
    pub aspect: f32,
    pub view_proj: Mat4,
    pub eye: Vec3,
    pub dist: f32,
    /// degrees — overridable (menu backdrop uses a slow orbit)
    pub pitch_deg: f32,
    pub yaw_deg: f32,
}

impl Camera {
    pub fn new(aspect: f32) -> Camera {
        Camera {
            target: Vec3::ZERO,
            smooth: Vec3::ZERO,
            aspect,
            view_proj: Mat4::IDENTITY,
            eye: Vec3::ZERO,
            dist: crate::consts::CAM_DIST,
            pitch_deg: crate::consts::CAM_PITCH_DEG,
            yaw_deg: crate::consts::CAM_YAW_DEG,
        }
    }

    pub fn update(&mut self, target: Vec3, dt: f32) {
        self.target = target;
        let t = (dt * crate::consts::CAM_SMOOTH).min(1.0);
        self.smooth = self.smoodge(self.target, t);
        let yaw = self.yaw_deg.to_radians();
        let pitch = self.pitch_deg.to_radians();
        let dir = Vec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
        );
        self.eye = self.smooth + dir * self.dist;
        let view = Mat4::look_at_rh(self.eye, self.smooth, Vec3::Y);
        let proj = Mat4::perspective_rh(
            crate::consts::CAM_FOV_DEG.to_radians(),
            self.aspect.max(0.1),
            0.1,
            160.0,
        );
        self.view_proj = proj * view;
    }

    fn smoodge(&self, target: Vec3, t: f32) -> Vec3 {
        self.smooth + (target - self.smooth) * t
    }

    /// Project a world position to pixel coordinates (returns None if behind camera).
    pub fn world_to_screen(&self, world: Vec3, w: f32, h: f32) -> Option<(f32, f32)> {
        let clip = self.view_proj * Vec4::new(world.x, world.y, world.z, 1.0);
        if clip.w <= 0.001 {
            return None;
        }
        let ndc = glam::Vec2::new(clip.x, clip.y) / clip.w;
        Some(((ndc.x * 0.5 + 0.5) * w, (1.0 - (ndc.y * 0.5 + 0.5)) * h))
    }
}
