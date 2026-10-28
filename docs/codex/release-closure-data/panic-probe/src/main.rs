use block_buffer::{generic_array::typenum::U4, EagerBuffer, LazyBuffer};
use std::panic::{catch_unwind, AssertUnwindSafe};
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut cases = 0;
    for n in 0..4 {
        for len in 1..17 {
            let mut b = EagerBuffer::<U4>::new(&vec![1; n]);
            let before = b.get_pos();
            let result = catch_unwind(AssertUnwindSafe(|| {
                b.digest_blocks(&vec![255; len], |_| panic!("compress"))
            }));
            if result.is_err() {
                assert_eq!(b.get_pos(), before);
                b.digest_blocks(&[1; 8], |_| {});
                cases += 1;
            }
            let mut b = EagerBuffer::<U4>::new(&vec![1; n]);
            let before = b.get_pos();
            assert!(catch_unwind(AssertUnwindSafe(|| b.digest_pad(
                255,
                &[255; 4],
                |_| panic!("pad")
            )))
            .is_err());
            assert_eq!(b.get_pos(), before);
            b.reset();
            cases += 1;
            let mut b = EagerBuffer::<U4>::new(&vec![1; n]);
            let before = b.get_pos();
            let mut data = [0; 16];
            assert!(
                catch_unwind(AssertUnwindSafe(|| b.set_data(&mut data, |blocks| {
                    for block in blocks {
                        block.fill(255)
                    }
                    panic!("generate")
                })))
                .is_err()
            );
            assert_eq!(b.get_pos(), before);
            cases += 1;
        }
    }
    for n in 0..=4 {
        let mut b = LazyBuffer::<U4>::new(&vec![1; n]);
        let before = b.get_pos();
        assert!(catch_unwind(AssertUnwindSafe(
            || b.digest_blocks(&[255; 8], |_| panic!("lazy"))
        ))
        .is_err());
        assert_eq!(b.get_pos(), before);
        cases += 1;
    }
    println!("{cases} caught-panic cases preserved cursor invariants; native only, not Miri or a universal safety proof");
}
