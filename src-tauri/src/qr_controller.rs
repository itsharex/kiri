//! Bounded transient scan ownership, independent of native windows and storage.
use crate::qr::QrCode;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

#[derive(Clone)]
pub(crate) struct Scan {
    pub id: uuid::Uuid,
    pub capture_id: Option<uuid::Uuid>,
    pub asset_id: Option<uuid::Uuid>,
    pub library_identity: (uuid::Uuid, uuid::Uuid),
    pub png: Arc<[u8]>,
    pub codes: Vec<QrCode>,
}

#[derive(Default)]
pub struct QrRequests {
    pub(crate) scans: Mutex<HashMap<String, Scan>>,
    busy: AtomicBool,
}

pub(crate) fn image_owner_matches(label: &str, asset: uuid::Uuid) -> bool {
    label == "library" || label == format!("editor-{asset}")
}
impl QrRequests {
    pub fn clear(&self, label: &str) {
        self.scans.lock().unwrap().remove(label);
    }

    pub(crate) fn begin(self: &Arc<Self>, label: String, scan: Scan) -> Result<Permit, String> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("Another QR image is being recognized. Try again shortly.".into());
        }
        self.scans.lock().unwrap().insert(label, scan);
        Ok(Permit(self.clone()))
    }

    pub(crate) fn cancel(&self, label: &str, id: uuid::Uuid) {
        let mut scans = self.scans.lock().unwrap();
        if scans.get(label).is_some_and(|s| s.id == id) {
            scans.remove(label);
        }
    }

    pub(crate) fn publish(
        &self,
        label: &str,
        id: uuid::Uuid,
        png: Arc<[u8]>,
        codes: Vec<QrCode>,
    ) -> Result<(), String> {
        let mut scans = self.scans.lock().unwrap();
        let slot = scans
            .get_mut(label)
            .filter(|s| s.id == id)
            .ok_or("The QR request was canceled.")?;
        slot.png = png;
        slot.codes = codes;
        Ok(())
    }
}
pub(crate) struct Permit(Arc<QrRequests>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scan() -> Scan {
        Scan {
            id: uuid::Uuid::new_v4(),
            capture_id: Some(uuid::Uuid::new_v4()),
            asset_id: None,
            library_identity: (uuid::Uuid::new_v4(), uuid::Uuid::new_v4()),
            png: Arc::from([]),
            codes: vec![],
        }
    }
    #[test]
    fn saved_images_are_scoped_to_the_library_or_the_matching_editor() {
        let id = uuid::Uuid::new_v4();
        assert!(image_owner_matches("library", id));
        assert!(image_owner_matches(&format!("editor-{id}"), id));
        assert!(!image_owner_matches("overlay", id));
        assert!(!image_owner_matches(
            &format!("editor-{}", uuid::Uuid::new_v4()),
            id
        ));
        assert!(!image_owner_matches(&format!("viewer-{id}"), id));
    }
    #[test]
    fn canceled_and_destroyed_owners_cannot_publish_or_escape_single_flight() {
        let requests = Arc::new(QrRequests::default());
        let scan = scan();
        let permit = requests.begin("overlay".into(), scan.clone()).unwrap();
        requests.cancel("overlay", scan.id);
        assert!(requests
            .publish("overlay", scan.id, Arc::from([1u8]), vec![])
            .is_err());
        assert!(requests.begin("library".into(), scan.clone()).is_err());
        drop(permit);
        let permit = requests.begin("library".into(), scan.clone()).unwrap();
        requests.clear("library");
        assert!(requests
            .publish("library", scan.id, Arc::from([1u8]), vec![])
            .is_err());
        drop(permit);
        assert!(requests.begin("library".into(), scan).is_ok());
    }
    #[test]
    fn other_owner_and_stale_request_ids_cannot_cancel_or_replace_current_scan() {
        let requests = Arc::new(QrRequests::default());
        let old = scan();
        let next = scan();
        drop(requests.begin("library".into(), old.clone()).unwrap());
        let permit = requests.begin("library".into(), next.clone()).unwrap();
        requests.cancel("overlay", next.id);
        requests.cancel("library", old.id);
        assert!(requests
            .publish("library", old.id, Arc::from([1u8]), vec![])
            .is_err());
        requests
            .publish("library", next.id, Arc::from([2u8]), vec![])
            .unwrap();
        assert_eq!(&*requests.scans.lock().unwrap()["library"].png, &[2]);
        drop(permit);
    }
}
