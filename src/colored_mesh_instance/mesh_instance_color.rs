use bevy::{
    ecs::reflect::ReflectComponent,
    prelude::{Color, Component, Deref, DerefMut, Reflect},
};

#[derive(Debug, Default, Copy, Clone, Deref, DerefMut, Component, Reflect)]
#[reflect(Component)]
pub struct InstanceColor(pub Color);

impl From<Color> for InstanceColor {
    fn from(color: Color) -> Self {
        InstanceColor(color)
    }
}

impl From<InstanceColor> for Color {
    fn from(color: InstanceColor) -> Self {
        color.0
    }
}
