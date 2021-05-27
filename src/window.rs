use winit::{event::{DeviceEvent, ElementState, Event, KeyboardInput, StartCause, VirtualKeyCode, WindowEvent}, event_loop::{ControlFlow, EventLoop}, window::WindowBuilder};

use super::engine::VulkanApp;

pub fn start() {

    let event_loop = EventLoop::new();
    //window/winit initalization
    let window = WindowBuilder::new()
    .with_title("test")
    .with_resizable(true)
    .build(&event_loop)
    .unwrap();

    let mut a = VulkanApp::new(&event_loop);

    let mut framenumber = 0;

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
                _ => (),
            },
            _ => (),
        },
        Event::MainEventsCleared => {
            a.draw(framenumber);
            framenumber = framenumber + 1;
        }
        _ => (),
    });



}

