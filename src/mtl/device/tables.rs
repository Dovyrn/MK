use super::{Device, id};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTLAllocation, MTLComputePipelineState, MTLIntersectionFunctionTable,
    MTLIntersectionFunctionTableDescriptor, MTLVisibleFunctionTable,
    MTLVisibleFunctionTableDescriptor,
};

impl Device {
    pub fn create_function_table(
        &mut self,
        pipeline: u32,
        functions: &[&str],
        intersection: bool,
    ) -> Result<u32, String> {
        let state = &self.res.pipelines.get(pipeline).state;
        let handles = functions
            .iter()
            .map(|f| {
                state
                    .functionHandleWithName(&NSString::from_str(f))
                    .ok_or(format!("{f} is not linked into the pipeline"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (table, handle): (Retained<ProtocolObject<dyn MTLAllocation>>, _) = if intersection {
            let desc = MTLIntersectionFunctionTableDescriptor::new();
            desc.setFunctionCount(handles.len());
            let table = state
                .newIntersectionFunctionTableWithDescriptor(&desc)
                .ok_or("intersection function table allocation failed")?;
            for (i, h) in handles.iter().enumerate() {
                table.setFunction_atIndex(Some(h), i);
            }
            let handle = id(table.gpuResourceID());
            (ProtocolObject::from_retained(table), handle)
        } else {
            let desc = MTLVisibleFunctionTableDescriptor::new();
            unsafe { desc.setFunctionCount(handles.len()) };
            let table = state
                .newVisibleFunctionTableWithDescriptor(&desc)
                .ok_or("visible function table allocation failed")?;
            for (i, h) in handles.iter().enumerate() {
                unsafe { table.setFunction_atIndex(Some(h), i) };
            }
            let handle = id(table.gpuResourceID());
            (ProtocolObject::from_retained(table), handle)
        };
        self.residency.add(&table);
        let id = self.res.tables.insert((table, handle));
        self.res.table.functions.set(id, ());
        Ok(id)
    }

    pub fn destroy_function_table(&mut self, table: u32) {
        self.res.table.functions.clear(table);
        let (table, _) = self.res.tables.remove(table);
        self.defer(move |r| r.remove(&table));
    }

    pub fn function_table_handle(&self, table: u32) -> u64 {
        self.res.tables.get(table).1
    }
}
