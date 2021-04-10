use crate::graphics::GraphicsState;

pub trait GameState {
    fn update(&mut self, window: &glfw::Window, dt: std::time::Duration);
    fn render(&self, graphics: &GraphicsState) -> Result<(), wgpu::SwapChainError>;
}
