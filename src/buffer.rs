use bytemuck::NoUninit;
use wgpu::{Buffer, BufferDescriptor, BufferUsages, Device, Queue};

/// A GPU buffer that is reallocated when an update requires more space.
pub struct DynamicBuffer {
    /// The GPU buffer. When size changes, this handle is replaced.
    pub inner: Buffer,
}

impl DynamicBuffer {
    pub fn new(device: &Device, size: u64, usage: BufferUsages) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: false,
        });
        Self { inner: buffer }
    }

    /// Creates a new buffer with the given size and replace the current one.
    /// Data from the old buffer is not copied.
    fn resize(&mut self, device: &Device, size: u64) {
        self.inner = device.create_buffer(&BufferDescriptor {
            label: None,
            size,
            usage: self.inner.usage(),
            mapped_at_creation: false,
        });
    }

    /// Queue a buffer data transfer. Reallocates a new buffer if too small.
    pub fn write_slice<A: NoUninit>(&mut self, device: &Device, queue: &Queue, data: &[A]) {
        let contents: &[u8] = bytemuck::cast_slice(data);
        if self.inner.size() < contents.len() as u64 {
            self.resize(device, contents.len() as u64);
        }
        queue.write_buffer(&self.inner, 0, contents);
    }
}
