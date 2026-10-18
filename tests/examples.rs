#![allow(clippy::duplicate_mod)]

#[path = "../examples/adopt.rs"]
mod adopt;
#[path = "../examples/atomics.rs"]
mod atomics;
#[path = "../examples/compute.rs"]
mod compute;
#[path = "../examples/copies.rs"]
mod copies;
#[path = "../examples/handles.rs"]
mod handles;
#[path = "../examples/images.rs"]
mod images;
#[path = "../examples/imports.rs"]
mod imports;
#[path = "../examples/indirect.rs"]
mod indirect;
#[path = "../examples/mesh.rs"]
mod mesh;
#[path = "../examples/native.rs"]
mod native;
#[path = "../examples/present.rs"]
mod present;
#[path = "../examples/queries.rs"]
mod queries;
#[path = "../examples/queues.rs"]
mod queues;
#[path = "../examples/reads.rs"]
mod reads;
#[path = "../examples/render.rs"]
mod render;
#[path = "../examples/residency.rs"]
mod residency;
#[path = "../examples/resolve.rs"]
mod resolve;
#[path = "../examples/shared.rs"]
mod shared;
#[path = "../examples/slots.rs"]
mod slots;
#[path = "../examples/sources.rs"]
mod sources;
#[path = "../examples/subgroups.rs"]
mod subgroups;
#[path = "../examples/texture.rs"]
mod texture;
#[path = "../examples/threads.rs"]
mod threads;
#[path = "../examples/tiles.rs"]
mod tiles;
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
    adopt, atomics, compute, copies, handles, images, imports, indirect, mesh, native, present,
    queries, queues, reads, render, residency, resolve, shared, slots, sources, subgroups, texture,
    threads, tiles, uploads, view, viewports
);
