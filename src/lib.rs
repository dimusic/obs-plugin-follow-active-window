use std::time::{Duration, Instant};

use active_win_pos_rs::{get_active_window, WindowPosition};
use log::info;
use obs_wrapper::{
    graphics::*, log::Logger, obs_register_module, obs_string, prelude::*, properties::*, source::*,
};

fn smooth_step(x: f32) -> f32 {
    let t = ((x / 1.).max(0.)).min(1.);
    t * t * (3. - 2. * t)
}

struct FollowActiveWindowFilter {
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

impl FollowActiveWindowFilter {
    fn update_active_window_data(&mut self, window_position: &WindowPosition) {
        let window_zoom = ((window_position.width / (self.screen_width as f64))
            .max(window_position.height / (self.screen_height as f64))
            as f64
            + self.padding)
            .max(self.internal_zoom)
            .min(1.);

        let win_x = if window_position.x < 0.0 {
            0.0
        } else {
            window_position.x
        };
        let win_y = if window_position.y < 0.0 {
            0.0
        } else {
            window_position.y
        };

        if win_x > (self.screen_width + self.screen_x) as f64
            || win_x < self.screen_x as f64
            || win_y < self.screen_y as f64
            || win_y > (self.screen_height + self.screen_y) as f64
        {
            if self.target_zoom != 1. && self.target.x() != 0. && self.target.y() != 0. {
                self.progress = 0.;
                self.from_zoom = self.current_zoom;
                self.target_zoom = 1.;

                self.from.set(self.current.x(), self.current.y());
                self.target.set(0., 0.);
            }
        } else {
            let x = (win_x + (window_position.width / 2.) - (self.screen_x as f64))
                / (self.screen_width as f64);
            let y = (win_y + (window_position.height / 2.) - (self.screen_y as f64))
                / (self.screen_height as f64);

            let target_x = (x - (0.5 * window_zoom as f64))
                .min(1. - window_zoom as f64)
                .max(0.);

            let target_y = (y - (0.5 * window_zoom as f64))
                .min(1. - window_zoom as f64)
                .max(0.);

            if (target_y - self.target.y() as f64).abs() > 0.001
                || (target_x - self.target.x() as f64).abs() > 0.001
                || (window_zoom - self.target_zoom).abs() > 0.001
            {
                self.progress = 0.;

                self.from_zoom = self.current_zoom;
                self.target_zoom = window_zoom;

                self.from.set(self.current.x(), self.current.y());

                self.target.set(target_x as f32, target_y as f32);
            }
        }
    }
}

struct TheModule {
    context: ModuleContext,
}

impl Sourceable for FollowActiveWindowFilter {
    fn get_id() -> ObsString {
        obs_string!("follow_active_window_filter")
    }

    fn get_type() -> SourceType {
        SourceType::FILTER
    }

    fn create(create: &mut CreatableSourceContext<Self>, mut source: SourceContext) -> Self {
        let mut effect = GraphicsEffect::from_effect_string(
            obs_string!(include_str!("./crop_filter.effect")),
            obs_string!("crop_filter.effect"),
        )
        .expect("Could not load crop filter effect!");

        let settings = &mut create.settings;

        if let (
            Some(image),
            Some(add_val),
            Some(base_dimension),
            Some(base_dimension_i),
            Some(mul_val),
        ) = (
            effect.get_effect_param_by_name(obs_string!("image")),
            effect.get_effect_param_by_name(obs_string!("add_val")),
            effect.get_effect_param_by_name(obs_string!("base_dimension")),
            effect.get_effect_param_by_name(obs_string!("base_dimension_i")),
            effect.get_effect_param_by_name(obs_string!("mul_val")),
        ) {
            let zoom = 1. / settings.get(obs_string!("zoom")).unwrap_or(1.);

            let sampler = GraphicsSamplerState::from(GraphicsSamplerInfo::default());

            let screen_width = settings.get(obs_string!("screen_width")).unwrap_or(1920);
            let screen_height = settings.get(obs_string!("screen_height")).unwrap_or(1080);

            let screen_x = settings.get(obs_string!("screen_x")).unwrap_or(0);
            let screen_y = settings.get(obs_string!("screen_y")).unwrap_or(0);

            let animation_time = settings.get(obs_string!("animation_time")).unwrap_or(0.3);

            let drawing_technique = settings
                .get(obs_string!("drawing_technique"))
                .unwrap_or(0_i64);
            //.unwrap_or(String::from("DrawUndistort"));

            source.update_source_settings(settings);

            let _ = Logger::new().init();

            return Self {
                source,
                effect,
                add_val,
                mul_val,

                base_dimension,
                base_dimension_i,

                image,

                sampler,

                animation_time,

                current_zoom: zoom,
                from_zoom: zoom,
                target_zoom: zoom,
                internal_zoom: zoom,

                current: Vec2::new(0., 0.),
                from: Vec2::new(0., 0.),
                target: Vec2::new(0., 0.),
                padding: 0.1,

                progress: 1.,

                screen_height,
                screen_width,
                screen_x,
                screen_y,

                drawing_technique,

                window_check_delay: Duration::from_secs_f32(0.9),
                // window_check_delay: Duration::from_secs_f32(0.05),
                last_active_window_check: Instant::now(),
                last_window_id: None,
                last_window_pid: 0,
            };
        }

        panic!("Failed to find correct effect params!");
    }
}

impl GetNameSource for FollowActiveWindowFilter {
    fn get_name() -> ObsString {
        obs_string!("Follow Active Window Filter")
    }
}

impl GetPropertiesSource for FollowActiveWindowFilter {
    fn get_properties(&mut self) -> Properties {
        let mut properties = Properties::new();

        properties
            .add(
                obs_string!("zoom"),
                obs_string!("Amount to zoom in window"),
                NumberProp::new_float(0.001)
                    .with_range(1.0..=5.0)
                    .with_slider(),
            )
            .add(
                obs_string!("screen_x"),
                obs_string!("Offset relative to top left screen - x"),
                // NumberProp::new_int().with_range(1u32..=3840 * 3),
                NumberProp::new_int().with_range(-3000..3000).with_slider(),
            )
            .add(
                obs_string!("screen_y"),
                obs_string!("Offset relative to top left screen - y"),
                NumberProp::new_int().with_range(-3000..3000).with_slider(),
                // NumberProp::new_int().with_range(1u32..=3840 * 3),
            )
            .add(
                obs_string!("padding"),
                obs_string!("Padding around each window"),
                NumberProp::new_float(0.001)
                    .with_range(..=0.5)
                    .with_slider(),
            )
            .add(
                obs_string!("screen_width"),
                obs_string!("Screen width"),
                NumberProp::new_int().with_range(1u32..=3840 * 3),
            )
            .add(
                obs_string!("screen_height"),
                obs_string!("Screen height"),
                NumberProp::new_int().with_range(1u32..=3840 * 3),
            )
            .add(
                obs_string!("animation_time"),
                obs_string!("Animation Time (s)"),
                NumberProp::new_float(0.001).with_range(0.3..=10.),
            );

        let mut draw_technique_list_prop = properties.add_list::<i64>(
            obs_string!("drawing_technique"),
            obs_string!("Drawing Technique"),
            false,
        );
        draw_technique_list_prop.push(obs_string!("DrawUndistort"), 0_i64);
        draw_technique_list_prop.push(obs_string!("DrawAlphaDivide"), 1_i64);
        draw_technique_list_prop.push(obs_string!("Draw"), 2_i64);

        properties
    }
}

impl VideoTickSource for FollowActiveWindowFilter {
    fn video_tick(&mut self, seconds: f32) {
        let data = self;

        if data.last_active_window_check.elapsed() > data.window_check_delay {
            match get_active_window() {
                Ok(active_window) => {
                    data.last_window_id = Some(active_window.window_id.clone());
                    data.last_window_pid = active_window.process_id;

                    let win_position = active_window.position;
                    info!(
                        "id: {}; pid: {}; position: {:?}",
                        active_window.window_id, active_window.process_id, win_position
                    );
                    data.update_active_window_data(&win_position);
                }
                Err(e) => info!("failed to get window position: {:?}", e),
            }

            data.last_active_window_check = Instant::now();
        }

        data.progress = (data.progress + seconds as f64 / data.animation_time).min(1.);

        let adjusted_progress = smooth_step(data.progress as f32);

        data.current.set(
            data.from.x() + (data.target.x() - data.from.x()) * adjusted_progress,
            data.from.y() + (data.target.y() - data.from.y()) * adjusted_progress,
        );

        data.current_zoom =
            data.from_zoom + (data.target_zoom - data.from_zoom) * adjusted_progress as f64;
    }
}

impl VideoRenderSource for FollowActiveWindowFilter {
    fn video_render(&mut self, _context: &mut GlobalContext, render: &mut VideoRenderContext) {
        let data = self;

        let effect = &mut data.effect;
        let source = &mut data.source;
        let param_add = &mut data.add_val;

        let param_mul = &mut data.mul_val;

        let param_base = &mut data.base_dimension;
        let param_base_i = &mut data.base_dimension_i;

        let image = &mut data.image;
        let sampler = &mut data.sampler;

        let current = &mut data.current;

        let zoom = data.current_zoom as f32;

        let mut target_cx: u32 = 1;
        let mut target_cy: u32 = 1;

        let cx = source.get_base_width();
        let cy = source.get_base_height();

        source.do_with_target(|target| {
            target_cx = target.get_base_width();
            target_cy = target.get_base_height();
        });

        let drawing_technique = match data.drawing_technique {
            0 => obs_string!("DrawUndistort"),
            1 => obs_string!("DrawAlphaDivide"),
            2 => obs_string!("Draw"),
            _ => obs_string!("DrawUnidistort"),
        };

        source.process_filter_tech(
            render,
            effect,
            (target_cx, target_cy),
            GraphicsColorFormat::RGBA,
            GraphicsAllowDirectRendering::NoDirectRendering,
            drawing_technique,
            |context, _effect| {
                param_add.set_vec2(context, &Vec2::new(current.x(), current.y()));
                param_mul.set_vec2(context, &Vec2::new(zoom, zoom));

                param_base.set_vec2(context, &Vec2::new(cx as _, cy as _));
                param_base_i.set_vec2(context, &Vec2::new(1. / (cx as f32), 1. / (cy as f32)));

                image.set_next_sampler(context, sampler);
            },
        );
    }
}

impl UpdateSource for FollowActiveWindowFilter {
    fn update(&mut self, settings: &mut DataObj, _context: &mut GlobalContext) {
        let data = self;

        if let Some(zoom) = settings.get::<f64>(obs_string!("zoom")) {
            info!("zoom: {}", zoom);
            data.from_zoom = data.current_zoom;
            data.internal_zoom = 1. / zoom;
            data.target_zoom = 1. / zoom;
        }

        if let Some(screen_width) = settings.get(obs_string!("screen_width")) {
            info!("screen_width: {}", screen_width);
            data.screen_width = screen_width;
        }

        if let Some(padding) = settings.get(obs_string!("padding")) {
            info!("padding: {}", padding);
            data.padding = padding;
        }

        if let Some(animation_time) = settings.get(obs_string!("animation_time")) {
            info!("animation_time: {}", animation_time);
            data.animation_time = animation_time;
        }

        if let Some(screen_height) = settings.get(obs_string!("screen_height")) {
            info!("screen_height: {}", screen_height);
            data.screen_height = screen_height;
        }

        if let Some(screen_x) = settings.get(obs_string!("screen_x")) {
            info!("screen_x: {}", screen_x);
            data.screen_x = screen_x;
        }

        if let Some(screen_y) = settings.get(obs_string!("screen_y")) {
            info!("screen_y: {}", screen_y);
            data.screen_y = screen_y;
        }

        if let Some(drawing_technique) = settings.get(obs_string!("drawing_technique")) {
            info!("drawing_technique: {}", drawing_technique);
            data.drawing_technique = drawing_technique;
        }
    }
}

impl Module for TheModule {
    fn new(context: ModuleContext) -> Self {
        Self { context }
    }

    fn get_ctx(&self) -> &ModuleContext {
        &self.context
    }

    fn load(&mut self, load_context: &mut LoadContext) -> bool {
        let source = load_context
            .create_source_builder::<FollowActiveWindowFilter>()
            .enable_get_name()
            .enable_get_properties()
            .enable_update()
            .enable_video_render()
            .enable_video_tick()
            .build();

        load_context.register_source(source);

        true
    }

    fn description() -> ObsString {
        obs_string!("Zoom in and follow currently focused window.")
    }

    fn name() -> ObsString {
        obs_string!("Follow Active Window Filter")
    }

    fn author() -> ObsString {
        obs_string!("Dmitry Malkov")
    }
}

obs_register_module!(TheModule);
