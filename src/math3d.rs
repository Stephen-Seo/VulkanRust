use crate::ffi;

type Vec2f = [f32; 2];
type Vec3f = [f32; 3];

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Vertex {
    pub pos: Vec2f,
    pub color: Vec3f,
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            pos: [0.0, 0.0],
            color: [0.0, 0.0, 0.0],
        }
    }
}

#[allow(dead_code)]
impl Vertex {
    pub fn new(pos: Vec2f, color: Vec3f) -> Self {
        Self { pos, color }
    }

    pub const fn pos_offset() -> usize {
        0
    }

    pub const fn color_offset() -> usize {
        let mut offset = std::mem::size_of::<Vec2f>();
        let alignment = std::mem::align_of::<Vec3f>();
        while offset % alignment != 0 {
            offset += 1;
        }

        offset
    }

    pub const fn get_binding_description() -> ffi::VkVertexInputBindingDescription {
        let mut bind_desc: ffi::VkVertexInputBindingDescription = unsafe { std::mem::zeroed() };

        bind_desc.binding = 0;
        bind_desc.stride = std::mem::size_of::<Self>() as u32;
        bind_desc.inputRate = ffi::VkVertexInputRate_VK_VERTEX_INPUT_RATE_VERTEX;

        bind_desc
    }

    pub const fn get_attribute_descriptions() -> [ffi::VkVertexInputAttributeDescription; 2] {
        let mut attr_descs: [ffi::VkVertexInputAttributeDescription; 2] =
            unsafe { std::mem::zeroed() };

        attr_descs[0].binding = 0;
        attr_descs[0].location = 0;
        attr_descs[0].format = ffi::VkFormat_VK_FORMAT_R32G32_SFLOAT;
        attr_descs[0].offset = Self::pos_offset() as u32;

        attr_descs[1].binding = 0;
        attr_descs[1].location = 1;
        attr_descs[1].format = ffi::VkFormat_VK_FORMAT_R32G32B32_SFLOAT;
        attr_descs[1].offset = Self::color_offset() as u32;

        attr_descs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets() {
        let mut vertex = Vertex {
            pos: [1.0, 2.0],
            color: [3.0, 4.0, 5.0],
        };

        let root_ptr: *const f32 = &vertex as *const Vertex as *const f32;

        let pos_offset = Vertex::pos_offset();
        assert!(pos_offset + 4 <= std::mem::size_of::<Vertex>());

        let pos_0_ptr = unsafe { root_ptr.byte_add(pos_offset) };
        assert_eq!(unsafe { *pos_0_ptr }, vertex.pos[0]);

        assert!(pos_offset + 8 <= std::mem::size_of::<Vertex>());
        let pos_1_ptr = unsafe { root_ptr.byte_add(pos_offset + 4) };
        assert_eq!(unsafe { *pos_1_ptr }, vertex.pos[1]);

        let color_offset = Vertex::color_offset();
        assert!(color_offset + 4 <= std::mem::size_of::<Vertex>());

        let col_0_ptr = unsafe { root_ptr.byte_add(color_offset) };
        assert_eq!(unsafe { *col_0_ptr }, vertex.color[0]);

        assert!(color_offset + 8 <= std::mem::size_of::<Vertex>());
        let col_1_ptr = unsafe { root_ptr.byte_add(color_offset + 4) };
        assert_eq!(unsafe { *col_1_ptr }, vertex.color[1]);

        assert!(color_offset + 12 <= std::mem::size_of::<Vertex>());
        let col_2_ptr = unsafe { root_ptr.byte_add(color_offset + 8) };
        assert_eq!(unsafe { *col_2_ptr }, vertex.color[2]);

        vertex.pos[0] = 0.123;
        vertex.pos[1] = 0.456;
        vertex.color[0] = 0.789;
        vertex.color[1] = 1.234;
        vertex.color[2] = 1.567;

        assert_eq!(unsafe { *pos_0_ptr }, vertex.pos[0]);
        assert_eq!(unsafe { *pos_1_ptr }, vertex.pos[1]);
        assert_eq!(unsafe { *col_0_ptr }, vertex.color[0]);
        assert_eq!(unsafe { *col_1_ptr }, vertex.color[1]);
        assert_eq!(unsafe { *col_2_ptr }, vertex.color[2]);
    }
}
