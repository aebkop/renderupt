use winit::event_loop::EventLoop;


mod engine;
fn main() {
    
    let event_loop = EventLoop::new();
    let a = engine::VulkanApp::new(&event_loop);
    a.run(event_loop)
}
