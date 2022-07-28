use bevy::{
    ecs::reflect::ReflectComponent,
    prelude::{Color, Component, Deref, DerefMut, Reflect},
};

#[derive(Debug, Default, Copy, Clone, Deref, DerefMut, Component, Reflect)]
#[reflect(Component)]
pub struct MeshInstanceColor(pub Color);

impl From<Color> for MeshInstanceColor {
    fn from(color: Color) -> Self {
        MeshInstanceColor(color)
    }
}

impl From<MeshInstanceColor> for Color {
    fn from(color: MeshInstanceColor) -> Self {
        color.0
    }
}
