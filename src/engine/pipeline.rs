use std::{ffi::CString, mem::size_of};

use crate::engine::mesh;

use super::{device::Physical, renderpass::RenderPass};
use erupt::{vk::{self}};
use vk_shader_macros::include_glsl;
const FRAG: &[u32] = include_glsl!("src/shaders/colored-triangle.frag", kind: frag);
const TRIMESH: &[u32] = include_glsl!("src/shaders/trimesh.vert");

pub struct PipelineStruct {
    pub pipelines: Vec<vk::Pipeline>,
    pub pipeline_layout: vk::PipelineLayout
}

impl PipelineStruct {
    pub fn new(physical: &Physical, render_pass: &RenderPass) -> Self {
                //Pipeline starts here
        //Shader Modules
        let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(FRAG);
        let frag_module = unsafe {  physical.device.create_shader_module(&module_info, None, None) }.unwrap();
        let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(TRIMESH);
        let entry_point = CString::new("main").unwrap();
        let tri_mesh =
            unsafe {  physical.device.create_shader_module(&module_info, None, None) }.unwrap();

    
        let shader_stages = vec![
            vk::PipelineShaderStageCreateInfoBuilder::new()
                .stage(vk::ShaderStageFlagBits::VERTEX)
                .module(tri_mesh)
                .name(&entry_point),
            vk::PipelineShaderStageCreateInfoBuilder::new()
                .stage(vk::ShaderStageFlagBits::FRAGMENT)
                .module(frag_module)
                .name(&entry_point),
        ];
        //like openGL VAO, not using it atm
        let vertex_desc = mesh::VertexDesc::new();
        let vertex_input = vk::PipelineVertexInputStateCreateInfoBuilder::new()
            .vertex_attribute_descriptions(&vertex_desc.attributes)
            .vertex_binding_descriptions(&vertex_desc.bindings);

        //what sort of topology drawn e.g triangles or lines or whatever
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let rasterizer = vk::PipelineRasterizationStateCreateInfoBuilder::new()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        let multisampling = vk::PipelineMultisampleStateCreateInfoBuilder::new()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlagBits::_1);

        let color_blend_attachments = vec![vk::PipelineColorBlendAttachmentStateBuilder::new()
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
            )
            .blend_enable(false)];
        let color_blending = vk::PipelineColorBlendStateCreateInfoBuilder::new()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments);

        let viewports = vec![vk::ViewportBuilder::new()
            .x(0.0)
            .y(0.0)
            .width(physical.surface_caps.current_extent.width as f32)
            .height(physical.surface_caps.current_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)];
        let scissors = vec![vk::Rect2DBuilder::new()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(physical.surface_caps.current_extent)];
        let viewport_state = vk::PipelineViewportStateCreateInfoBuilder::new()
            .viewports(&viewports)
            .scissors(&scissors);

        let pipeline_depth_stencil_info = vk::PipelineDepthStencilStateCreateInfoBuilder::new()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .depth_bounds_test_enable(false)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0)
            .stencil_test_enable(false);
        
        let push_constant = [vk::PushConstantRangeBuilder::new()
            .offset(0)
            .size(size_of::<[f32; 16]>() as u32)
            .stage_flags(vk::ShaderStageFlags::VERTEX)];

        let pipeline_layout_info = vk::PipelineLayoutCreateInfoBuilder::new()
            .push_constant_ranges(&push_constant);
        let pipeline_layout =
            unsafe { physical.device.create_pipeline_layout(&pipeline_layout_info, None, None) }.unwrap();

        let pipeline_infos = vec![
            vk::GraphicsPipelineCreateInfoBuilder::new()
                //Colored Triangle
                .stages(&shader_stages)
                .vertex_input_state(&vertex_input)
                .input_assembly_state(&input_assembly)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterizer)
                .multisample_state(&multisampling)
                .color_blend_state(&color_blending)
                .layout(pipeline_layout)
                .render_pass(render_pass.render_pass)
                .depth_stencil_state(&pipeline_depth_stencil_info)
                .subpass(0),
        ];

        let pipelines =
            unsafe { physical.device.create_graphics_pipelines(None, &pipeline_infos, None) }.unwrap();

        //delete shader modules now.
        unsafe { 
            physical.device.destroy_shader_module(Some(frag_module),None);
            physical.device.destroy_shader_module(Some(tri_mesh),None);
        };

        PipelineStruct {
            pipelines: pipelines,
            pipeline_layout
        }
    }
}

