use image::{DynamicImage, EncodableLayout, ImageBuffer, Luma, RgbaImage};
use moka::sync::Cache;
use std::{
    collections::HashSet,
    sync::{Arc, Condvar, Mutex},
};

/// Cache element.
#[derive(Clone)]
enum Item {
    /// Image loaded as-is.
    Dynamic(Arc<DynamicImage>),
    /// Image converted to RGBA8 format.
    Rgba(Arc<RgbaImage>),
    /// Image converted to Luma32 format.
    Luma32(Arc<ImageBuffer<Luma<f32>, Vec<f32>>>),
}

/// Possible formats for storing images. This is the same as [Item] enumeration but without the
/// data, and it is intended to be used as part of the cache key type.
#[derive(PartialEq, Eq, Hash)]
enum Format {
    Dynamic,
    Rgba,
    Luma32,
}

/// The cache key type, which identifies an image path and a data format once loaded.
#[derive(PartialEq, Eq, Hash)]
struct Key {
    /// Image path.
    source: String,
    /// Storage format once loaded and converted.
    format: Format,
}

/// Loads and cache images.
///
/// This struct can be cloned and shared accross threads, so all tasks in the program can share
/// efficiently the images.
///
/// Eventually, images can be put in a queue when requested and loaded by a worker thread monitoring
/// the queue. This is used for instance to avoid blocking the rendering if an image is not yet
/// available.
#[derive(Clone)]
pub struct ImageCache {
    /// Stores the images, referenced by a key.
    /// This is implemented using moka library.
    cache: Cache<Key, Item>,
    /// Images awaiting to be loaded by a worker thread.
    queue: Arc<Queue>,
}

/// Objects needed for image loading queue management.
struct Queue {
    inner: Mutex<HashSet<Key>>,
    cv: Condvar,
}

impl Queue {
    fn new() -> Self {
        Self {
            inner: Mutex::new(HashSet::new()),
            cv: Condvar::new(),
        }
    }
}

impl ImageCache {
    /// Constructs a new image cache with the given maximum capacity in bytes.
    pub fn new(cap: u64) -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(cap)
                .weigher(|_, item| -> u32 {
                    match item {
                        Item::Dynamic(image) => image.as_bytes().len().try_into().unwrap(),
                        Item::Rgba(image) => image.as_bytes().len().try_into().unwrap(),
                        Item::Luma32(image) => image.as_bytes().len().try_into().unwrap(),
                    }
                })
                .build(),
            queue: Arc::new(Queue::new()),
        }
    }

    /// Returns the requested image as a [DynamicImage], or None if it is not available in the
    /// cache. When the image is not available, it is put in the queue of the images to be loaded by
    /// a worker thread.
    pub fn need_dynamic(&self, source: &str) -> Option<Arc<DynamicImage>> {
        let key = Key {
            source: source.into(),
            format: Format::Dynamic,
        };
        if let Some(item) = self.cache.get(&key) {
            Some(match item {
                Item::Dynamic(image) => image,
                _ => panic!(),
            })
        } else {
            let mut q = self.queue.inner.lock().unwrap();
            if q.insert(key) {
                self.queue.cv.notify_one();
            }
            None
        }
    }

    /// Returns the requested image as a Luma32f image, or None if it is not available in the cache.
    /// When the image is not available, it is put in the queue of the images to be loaded by a
    /// worker thread.
    pub fn need_luma32(&self, source: &str) -> Option<Arc<ImageBuffer<Luma<f32>, Vec<f32>>>> {
        let key = Key {
            source: source.into(),
            format: Format::Luma32,
        };
        if let Some(item) = self.cache.get(&key) {
            Some(match item {
                Item::Luma32(image) => image,
                _ => panic!(),
            })
        } else {
            let mut q = self.queue.inner.lock().unwrap();
            if q.insert(key) {
                self.queue.cv.notify_one();
            }
            None
        }
    }

    /// Spawns a background thread that loads the queued requests into the cache.
    ///
    /// The thread blocks until requests are queued (e.g. via [`ImageCache::need_dynamic`]),
    /// then loads each of them into the cache. The loop runs for the lifetime of the process.
    pub fn spawn_loader(&self) -> std::thread::JoinHandle<()> {
        let cache = self.clone();
        std::thread::spawn(move || {
            loop {
                let keys = {
                    let mut q = cache.queue.inner.lock().unwrap();
                    while q.is_empty() {
                        q = cache.queue.cv.wait(q).unwrap();
                    }
                    std::mem::take(&mut (*q))
                };
                for key in keys {
                    println!("DEBUG thread loader picked {}", key.source);
                    match key.format {
                        Format::Dynamic => {
                            cache.get_dynamic(&key.source);
                        }
                        Format::Rgba => {
                            cache.get_rgba(&key.source);
                        }
                        Format::Luma32 => {
                            cache.get_luma32(&key.source);
                        }
                    }
                }
            }
        })
    }

    /// Returns the requested image as a [DynamicImage].
    pub fn get_dynamic(&self, source: &str) -> Arc<DynamicImage> {
        let item = self.cache.get_with(
            Key {
                source: source.into(),
                format: Format::Dynamic,
            },
            || {
                println!("DEBUG loading image {}", source);
                Item::Dynamic(Arc::new(image::open(source).unwrap()))
            },
        );
        match item {
            Item::Dynamic(image) => image,
            _ => panic!(),
        }
    }

    /// Returns the requested image converted to RGBA8 format.
    pub fn get_rgba(&self, source: &str) -> Arc<RgbaImage> {
        let item = self.cache.clone().get_with(
            Key {
                source: source.into(),
                format: Format::Rgba,
            },
            || {
                println!("DEBUG converting image to RGBA {}", source);
                let image = self.get_dynamic(source);
                Item::Rgba(Arc::new(image.to_rgba8()))
            },
        );
        match item {
            Item::Rgba(image) => image,
            _ => panic!(),
        }
    }

    /// Returns the requested image converted to Luma32f format.
    pub fn get_luma32(&self, source: &str) -> Arc<ImageBuffer<Luma<f32>, Vec<f32>>> {
        let item = self.cache.clone().get_with(
            Key {
                source: source.into(),
                format: Format::Luma32,
            },
            || {
                println!("DEBUG converting image to Luma32 {}", source);
                let image = self.get_dynamic(source);
                Item::Luma32(Arc::new(image.to_luma32f()))
            },
        );
        match item {
            Item::Luma32(image) => image,
            _ => panic!(),
        }
    }
}
