use std::time::{Duration, Instant};

use obs_wrapper::{source::SourceContext, graphics::{GraphicsEffect, GraphicsEffectVec2Param, GraphicsEffectTextureParam, GraphicsSamplerState, Vec2}};

pub struct Data {
    pub source: SourceContext,
    pub effect: GraphicsEffect,

    pub base_dimension: GraphicsEffectVec2Param,
    pub base_dimension_i: GraphicsEffectVec2Param,

    pub mul_val: GraphicsEffectVec2Param,
    pub add_val: GraphicsEffectVec2Param,

    pub image: GraphicsEffectTextureParam,

    pub sampler: GraphicsSamplerState,

    pub current: Vec2,
    pub from: Vec2,
    pub target: Vec2,

    pub animation_time: f64,

    pub current_zoom: f64,
    pub from_zoom: f64,
    pub target_zoom: f64,
    pub internal_zoom: f64,
    pub padding: f64,

    pub progress: f64,

    pub screen_width: u32,
    pub screen_height: u32,
    pub screen_x: u32,
    pub screen_y: u32,
    
    pub drawing_technique: i64,

    pub window_check_delay: Duration,
    pub last_active_window_check: Instant,
    pub last_window_id: Option<String>,
    pub last_window_pid: u64,
}
