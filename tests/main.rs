#[path = "support/host.rs"]
mod host;

mod adopt;
mod atomics;
mod compute;
mod copies;
mod handles;
mod images;
mod imports;
mod indirect;
mod mesh;
mod native;
mod present;
mod queries;
mod queues;
mod reads;
mod render;
mod residency;
mod resolve;
mod sets;
mod shared;
mod sources;
mod subgroups;
mod texture;
mod threads;
mod tiles;
mod uploads;
mod view;
mod viewports;

macro_rules! tests {
    ($($name:ident),*) => {
        $(
            #[test]
            fn $name() {
                $name::test();
            }
        )*
    };
}

tests!(
    adopt, atomics, compute, copies, handles, images, imports, indirect, mesh, native, present,
    queries, queues, reads, render, residency, resolve, sets, shared, sources, subgroups, texture,
    threads, tiles, uploads, view, viewports
);
