#[cfg(all(feature = "guest", not(feature = "native")))]
mod guest;
#[cfg(all(feature = "guest", not(feature = "native")))]
pub use guest::*;

#[cfg(feature = "native")]
mod native;
#[cfg(feature = "native")]
pub mod components {
    pub mod app {
        pub use ambient_core::{name, ui_scene, window_logical_size, window_physical_size, window_scale_factor};
    }
    pub mod ecs {
        pub use ambient_core::hierarchy::{children, parent};
    }
    pub mod transform {
        pub use ambient_core::transform::{local_to_parent, local_to_world, mesh_to_local, mesh_to_world, rotation, scale, translation};
    }
    pub mod ui {
        pub use ambient_layout::{
            gpu_ui_size,
            guest_api::{
                align_horizontal_begin, align_horizontal_center, align_horizontal_end, align_vertical_begin, align_vertical_center,
                align_vertical_end, fit_horizontal_children, fit_horizontal_none, fit_horizontal_parent, fit_vertical_children,
                fit_vertical_none, fit_vertical_parent, layout_bookcase, layout_dock, layout_flow, layout_width_to_children,
                orientation_horizontal, orientation_vertical,
            },
            height, is_book_file, mesh_to_local_from_size, min_height, min_width, space_between_items, width,
        };
        pub use ambient_rect::{background_color, border_color, border_radius, border_thickness, rect};
        pub use ambient_text::{font_size, text};
    }
    pub mod rendering {
        pub use ambient_renderer::color;
    }
}

#[cfg(feature = "guest")]
pub use ambient_api as api;
#[cfg(feature = "guest")]
pub use ambient_api::components::core as components;

#[cfg(feature = "guest")]
pub mod ecs {
    use ambient_api::ecs::SupportedComponentTypeGet;
    pub use ambient_api::{
        ecs::{Component, SupportedComponentTypeSet as ComponentValue, UntypedComponent}, prelude::{Entity, EntityId}
    };

    #[derive(Clone, Copy)]
    pub struct World;
    impl World {
        pub fn spawn(&self, entity: Entity) -> EntityId {
            ambient_api::entity::spawn(&entity)
        }
        pub fn despawn(&self, entity_id: EntityId) -> bool {
            ambient_api::entity::despawn(entity_id)
        }
        pub fn set<T: ComponentValue>(&self, entity_id: EntityId, component: Component<T>, value: T) -> Result<(), ECSError> {
            // TODO: set_component needs to return errors
            ambient_api::entity::set_component(entity_id, component, value);
            Ok(())
        }
        pub fn add_component<T: ComponentValue>(&self, entity_id: EntityId, component: Component<T>, value: T) -> Result<(), ECSError> {
            // TODO: add_component needs to return errors
            ambient_api::entity::add_component(entity_id, component, value);
            Ok(())
        }
        pub fn add_components(&self, entity_id: EntityId, components: Entity) -> Result<(), ECSError> {
            // TODO: add_components needs to return errors
            ambient_api::entity::add_components(entity_id, components);
            Ok(())
        }
        pub fn get<T: ComponentValue + SupportedComponentTypeGet>(
            &self,
            entity_id: EntityId,
            component: Component<T>,
        ) -> Result<T, ECSError> {
            ambient_api::entity::get_component(entity_id, component).ok_or_else(|| ECSError::EntityDoesntHaveComponent)
        }
        // TODO: This should actually return &T
        pub fn get_ref<T: ComponentValue + SupportedComponentTypeGet>(
            &self,
            entity_id: EntityId,
            component: Component<T>,
        ) -> Result<T, ECSError> {
            self.get(entity_id, component)
        }
        pub fn has_component<T: SupportedComponentTypeGet>(&self, entity_id: EntityId, component: Component<T>) -> bool {
            ambient_api::entity::has_component(entity_id, component)
        }
        pub fn resource<T: ComponentValue + SupportedComponentTypeGet>(&self, component: Component<T>) -> T {
            ambient_api::entity::get_component(ambient_api::entity::resources(), component).unwrap()
        }
    }
    #[derive(Debug)]
    pub enum ECSError {
        EntityDoesntHaveComponent,
        NoSuchEntity,
    }

    pub struct ComponentDesc(Box<dyn UntypedComponent>);
    impl ComponentDesc {
        pub fn index(&self) -> u32 {
            self.0.index()
        }
    }
    impl<T: 'static> From<Component<T>> for ComponentDesc {
        fn from(value: Component<T>) -> Self {
            Self(Box::new(value))
        }
    }
}
