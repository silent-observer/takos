pub mod block_allocator;
pub mod frame_allocator;
mod fresh_frame_allocator;
mod used_frame_allocator;

#[test_case]
fn test_allocator() {
    use crate::{print, println};
    use alloc::vec;

    print!("test_allocator... ");

    let v1 = vec![1, 2, 3, 4, 5];
    let v2 = vec![10, 11];
    assert_eq!(v1[0], 1);
    assert_eq!(v1[1], 2);
    assert_eq!(v1[2], 3);
    assert_eq!(v1[3], 4);
    assert_eq!(v1[4], 5);
    assert_eq!(v2[0], 10);
    assert_eq!(v2[1], 11);

    println!("[ok]");
}
