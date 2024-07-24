use std::sync::{Arc, RwLock};

use slint::{Model, ModelNotify, ModelTracker};


pub struct VecArcModel<T> {
    // the backing data, stored in a `RefCell` as this model can be modified
    array: Arc<RwLock<Vec<T>>>,
    // the ModelNotify will allow to notify the UI that the model changes
    notify: ModelNotify,
}

impl<T: Clone + 'static> Model for VecArcModel<T> {
    type Data = T;

    fn row_count(&self) -> usize {
        self.array.read().map_or(0, |v| v.len())
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.array.read().map_or(None, |v| v.get(row).cloned())
    }

    fn set_row_data(&self, row: usize, data: Self::Data) {
        let mut arr_lock = self.array.write().unwrap();
        (*arr_lock)[row] = data;
        // don't forget to call row_changed
        self.notify.row_changed(row);
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.notify
    }

    fn as_any(&self) -> &dyn core::any::Any {
        // a typical implementation just return `self`
        self
    }
}

// when modifying the model, we call the corresponding function in
// the ModelNotify
impl<T> VecArcModel<T> {
    pub fn new(vec: Vec<T>) -> Self {
        let notify = ModelNotify::default();
        VecArcModel {
            array: Arc::new(RwLock::new(vec)),
            notify
        }
    }
    pub fn clone(&self) -> Arc<RwLock<Vec<T>>> {
        self.array.clone()
    }
    /// Add a row at the end of the model
    pub fn push(&self, value: T) {
        let mut arr_lock = self.array.write().unwrap();
        (*arr_lock).push(value);
        self.notify.row_added(self.array.read().unwrap().len() - 1, 1)
    }

    /// Remove the row at the given index from the model
    pub fn remove(&self, index: usize) {
        let mut arr_lock = self.array.write().unwrap();
        (*arr_lock).remove(index);
        self.notify.row_removed(index, 1)
    }
}