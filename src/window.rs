use winit::{
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, StartCause, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
extern crate nalgebra as na;

use super::engine::VulkanApp;

const CAMERA_SPEED: f32 = 0.10;
pub fn start() {
    let event_loop = EventLoop::new();
    //window/winit initalization
    let window = WindowBuilder::new()
        .with_title("test")
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();

    let mut a = VulkanApp::new(&window);

    let mut framenumber = 0;

    let mut camera_pos: na::Point3<f32> = na::Point3::new(0.0, 0.0, 1.0);

    event_loop.run(move |event, _, control_flow| match event {
        Event::NewEvents(StartCause::Init) => {
            *control_flow = ControlFlow::Poll;
        }
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            _ => (),
        },
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::Key(KeyboardInput {
                virtual_keycode: Some(keycode),
                state,
                ..
            }) => match (keycode, state) {
                (VirtualKeyCode::Escape, ElementState::Released) => {
                    *control_flow = ControlFlow::Exit
                }
                (VirtualKeyCode::W, ElementState::Pressed) => {
                    camera_pos[2] += CAMERA_SPEED;
                }
                (VirtualKeyCode::A, ElementState::Pressed) => {
                    camera_pos[1] -= CAMERA_SPEED;
                }
                (VirtualKeyCode::S, ElementState::Pressed) => {
                    camera_pos[2] -= CAMERA_SPEED;
                }
                (VirtualKeyCode::D, ElementState::Pressed) => {
                    camera_pos[1] += CAMERA_SPEED;
                }
                _ => (),
            },
            _ => (),
        },
        Event::MainEventsCleared => {
            a.draw(framenumber, camera_pos);
            framenumber = framenumber + 1;
        }
        _ => (),
    });
}
