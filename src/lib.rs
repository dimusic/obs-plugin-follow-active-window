mod data;

use std::time::{Instant, Duration};

use active_win_pos_rs::{WindowPosition, get_active_window};
use data::Data;
use log::{info};
use obs_wrapper::{
    graphics::*,
    obs_register_module,
    obs_string,
    prelude::*,
    source::*, log::Logger
};

struct FollowActiveWindowFilter {
    context: ModuleContext,
}

fn smooth_step(x: f32) -> f32 {
    let t = ((x / 1.).max(0.)).min(1.);
    t * t * (3. - 2. * t)
}

fn update_active_window_data(data: &mut Data, win_position: &WindowPosition) -> () {
    let window_zoom = (
        (win_position.width / (data.screen_width as f64))
            .max(win_position.height / (data.screen_height as f64)) as f64
        + data.padding
    ).max(data.internal_zoom).min(1.);

    let win_x = if win_position.x < 0.0 { 0.0 } else { win_position.x };
    let win_y = if win_position.y < 0.0 { 0.0 } else { win_position.y };

    if win_x > (data.screen_width + data.screen_x) as f64
        || win_x < data.screen_x as f64
        || win_y < data.screen_y as f64
        || win_y > (data.screen_height + data.screen_y) as f64
    {
        if data.target_zoom != 1. && data.target.x() != 0. && data.target.y() != 0. {
            data.progress = 0.;
            data.from_zoom = data.current_zoom;
            data.target_zoom = 1.;

            data.from.set(data.current.x(), data.current.y());
            data.target.set(0., 0.);
        }
    } else {
        let x = (win_x + (win_position.width / 2.) - (data.screen_x as f64))
            / (data.screen_width as f64);
        let y = (win_y + (win_position.height / 2.) - (data.screen_y as f64))
            / (data.screen_height as f64);

        let target_x = (x - (0.5 * window_zoom as f64))
            .min(1. - window_zoom as f64)
            .max(0.);

        let target_y = (y - (0.5 * window_zoom as f64))
            .min(1. - window_zoom as f64)
            .max(0.);

        if (target_y - data.target.y() as f64).abs() > 0.001
            || (target_x - data.target.x() as f64).abs() > 0.001
            || (window_zoom - data.target_zoom).abs() > 0.001
        {
            data.progress = 0.;

            data.from_zoom = data.current_zoom;
            data.target_zoom = window_zoom;

            data.from.set(data.current.x(), data.current.y());

            data.target.set(target_x as f32, target_y as f32);
        }
    }
}

impl Sourceable for FollowActiveWindowFilter {
    fn get_id() -> ObsString {
        obs_string!("follow_active_window_filter")
    }
    fn get_type() -> SourceType {
        SourceType::FILTER
    }
}

impl GetNameSource<Data> for FollowActiveWindowFilter {
    fn get_name() -> ObsString {
        obs_string!("Follow Active Window")
    }
}

impl GetPropertiesSource<Data> for FollowActiveWindowFilter {
    fn get_properties(_data: &mut Option<Data>, properties: &mut Properties) {
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
                NumberProp::new_int().with_range(1u32..=3840 * 3),
            )
            .add(
                obs_string!("screen_y"),
                obs_string!("Offset relative to top left screen - y"),
                NumberProp::new_int().with_range(1u32..=3840 * 3),
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
            false
        );
        draw_technique_list_prop.push(obs_string!("DrawUndistort"), 0 as i64);
        draw_technique_list_prop.push(obs_string!("DrawAlphaDivide"), 1 as i64);
        draw_technique_list_prop.push(obs_string!("Draw"), 2 as i64);
    }

}

impl VideoTickSource<Data> for FollowActiveWindowFilter {
    fn video_tick(data: &mut Option<Data>, seconds: f32) {
        if let Some(data) = data {

            if data.last_active_window_check.elapsed() > data.window_check_delay {
                match get_active_window() {
                    Ok(active_window) => {
                        data.last_window_id = Some(active_window.window_id.clone());
                        data.last_window_pid = active_window.process_id;

                        let win_position = active_window.position;
                        // info!("id: {}; pid: {}; position: {:?}", active_window.window_id, active_window.process_id, win_position);
                        update_active_window_data(data, &win_position);
                    },
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
}

impl VideoRenderSource<Data> for FollowActiveWindowFilter {
    fn video_render(
        data: &mut Option<Data>,
        _context: &mut GlobalContext,
        render: &mut VideoRenderContext,
    ) {
        if let Some(data) = data {
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
}

impl CreatableSource<Data> for FollowActiveWindowFilter {
    fn create(create: &mut CreatableSourceContext<Data>, mut source: SourceContext) -> Data {
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

            let drawing_technique = settings.get(obs_string!("drawing_technique")).unwrap_or(0 as i64);
            //.unwrap_or(String::from("DrawUndistort"));

            source.update_source_settings(settings);

            let _ = Logger::new().init();

            return Data {
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

                drawing_technique: drawing_technique,
                
                // window_check_delay: Duration::from_secs_f32(0.9),
                window_check_delay: Duration::from_secs_f32(0.05),
                last_active_window_check: Instant::now(),
                last_window_id: None,
                last_window_pid: 0,
            };
        }

        panic!("Failed to find correct effect params!");
    }
}

impl UpdateSource<Data> for FollowActiveWindowFilter {
    fn update(data: &mut Option<Data>, settings: &mut DataObj, _context: &mut GlobalContext) {
        if let Some(data) = data {
            if let Some(zoom) = settings.get::<f64, _>(obs_string!("zoom")) {
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
}

impl Module for FollowActiveWindowFilter {
    fn new(context: ModuleContext) -> Self {
        Self { context }
    }
    fn get_ctx(&self) -> &ModuleContext {
        &self.context
    }

    fn load(&mut self, load_context: &mut LoadContext) -> bool {
        

        let source = load_context
            .create_source_builder::<FollowActiveWindowFilter, Data>()
            .enable_get_name()
            .enable_create()
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

obs_register_module!(FollowActiveWindowFilter);
