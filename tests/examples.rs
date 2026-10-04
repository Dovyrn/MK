#![allow(clippy::duplicate_mod)]

#[path = "../examples/adopt.rs"]
mod adopt;
#[path = "../examples/atomics.rs"]
mod atomics;
#[path = "../examples/blits.rs"]
mod blits;
#[path = "../examples/cache.rs"]
mod cache;
#[path = "../examples/compact.rs"]
mod compact;
#[path = "../examples/compute.rs"]
mod compute;
#[path = "../examples/handles.rs"]
mod handles;
#[path = "../examples/heap.rs"]
mod heap;
#[path = "../examples/images.rs"]
mod images;
#[path = "../examples/indirect.rs"]
mod indirect;
#[path = "../examples/mesh.rs"]
mod mesh;
#[path = "../examples/native.rs"]
mod native;
#[path = "../examples/queries.rs"]
mod queries;
#[path = "../examples/queues.rs"]
mod queues;
#[path = "../examples/raytrace.rs"]
mod raytrace;
#[path = "../examples/render.rs"]
mod render;
#[path = "../examples/resolve.rs"]
mod resolve;
#[path = "../examples/slots.rs"]
mod slots;
#[path = "../examples/sparse.rs"]
mod sparse;
#[path = "../examples/subgroups.rs"]
mod subgroups;
#[path = "../examples/texture.rs"]
mod texture;
#[path = "../examples/uploads.rs"]
mod uploads;
#[path = "../examples/view.rs"]
mod view;
#[path = "../examples/viewports.rs"]
mod viewports;

macro_rules! examples {
    ($($name:ident),*) => {
        $(
            #[test]
            fn $name() {
                $name::main();
            }
        )*
    };
}

examples!(
    adopt, atomics, blits, cache, compact, compute, handles, heap, images, indirect, mesh, native,
    queries, queues, raytrace, render, resolve, slots, sparse, subgroups, texture, uploads, view,
    viewports
);
