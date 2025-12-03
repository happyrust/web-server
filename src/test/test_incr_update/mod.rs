mod binary_patch;

use std::ops::RangeInclusive;

use crate::data_interface::tidb_manager::AiosDBManager;
use binary_patch::BinaryPatch;

const BASE_HEADER_BYTES: &[u8] =
    include_bytes!("../../../test_data/increment_patch/fake_pdms_base.bin");
const PATCH_SESNO12_BYTES: &[u8] =
    include_bytes!("../../../test_data/increment_patch/fake_pdms_sesno12.patchbin");
const PATCH_SESNO20_BYTES: &[u8] =
    include_bytes!("../../../test_data/increment_patch/fake_pdms_sesno20.patchbin");

#[cfg(feature = "sync_tests")]
mod watcher_flow {
    use std::sync::Arc;

    use crate::data_interface::tidb_manager::AiosDBManager;
    use crate::test::test_helper::get_test_ams_db_manager_async;
    use crate::test::test_query::init_test_surreal;

    #[tokio::test]
    async fn test_watch_update() {
        init_test_surreal().await;
        let mgr = Arc::new(get_test_ams_db_manager_async().await);
        futures::executor::block_on(async {
            AiosDBManager::exec_watcher(mgr.clone())
                .await
                .expect("watcher error");
        });
    }
}

#[test]
fn binary_patch_applies_new_sesno() {
    let patch = BinaryPatch::from_bytes(PATCH_SESNO12_BYTES).expect("parse patch");
    let patched = patch.apply_to_bytes(BASE_HEADER_BYTES);
    let (_, sesno) = parse_fake_header(&patched);
    assert_eq!(sesno, 12);
}

#[test]
fn binary_patch_generates_increment_range_from_patch() {
    let patch = BinaryPatch::from_bytes(PATCH_SESNO12_BYTES).expect("parse patch");
    let patched = patch.apply_to_bytes(BASE_HEADER_BYTES);
    let (_, sesno) = parse_fake_header(&patched);
    assert_eq!(
        AiosDBManager::compute_increment_range(6, sesno as i32, |candidate| Some(candidate)),
        Some(RangeInclusive::new(7, 12))
    );
}

#[test]
fn binary_patch_allows_zero_db_progress() {
    let patch = BinaryPatch::from_bytes(PATCH_SESNO20_BYTES).expect("parse patch");
    let patched = patch.apply_to_bytes(BASE_HEADER_BYTES);
    let (_, sesno) = parse_fake_header(&patched);
    let range =
        AiosDBManager::compute_increment_range(0, sesno as i32, |candidate| Some(candidate));
    assert_eq!(range, Some(RangeInclusive::new(1, 20)));
}

#[test]
fn binary_patch_respects_nearest_lookup_adjustments() {
    let patch = BinaryPatch::from_bytes(PATCH_SESNO20_BYTES).expect("parse patch");
    let patched = patch.apply_to_bytes(BASE_HEADER_BYTES);
    let (_, sesno) = parse_fake_header(&patched);
    let mut captured_candidate = None;
    let range = AiosDBManager::compute_increment_range(8, sesno as i32, |candidate| {
        captured_candidate = Some(candidate);
        Some(candidate + 2)
    });
    assert_eq!(captured_candidate, Some(9));
    assert_eq!(range, Some(RangeInclusive::new(11, 20)));
}

fn parse_fake_header(bytes: &[u8]) -> (u32, u32) {
    assert!(
        bytes.len() >= 12,
        "fake header must contain magic, db_no and sesno"
    );
    assert_eq!(&bytes[0..4], b"PDMS");
    let db_no = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let sesno = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    (db_no, sesno)
}
