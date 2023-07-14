use wgpu::BindGroup;

/// Contains the main [`BindGroup`]s used by Ambient's renderer.
#[derive(Debug, Clone, Copy)]
pub struct BindGroups<'a> {
    pub globals: &'a BindGroup,
    pub entities: &'a BindGroup,
    // Subset of `mesh_data`
    pub mesh_meta: &'a BindGroup,
}
