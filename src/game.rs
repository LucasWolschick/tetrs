use crate::graphics::GraphicsState;

pub enum StateChange {
    None,
    Quit,
    Push(Box<dyn GameState>),
    Pop,
    Swap(Box<dyn GameState>),
}

pub trait GameState {
    fn update(&mut self, window: &glfw::Window, dt: std::time::Duration) -> StateChange;
    fn render(&self, graphics: &GraphicsState) -> Result<(), wgpu::SwapChainError>;
}
