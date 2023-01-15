use std::fmt::Debug;

// TODO
pub fn assert_maximal_view_exists<T: Default + PartialEq, const N: usize>(views: &Vec<[T; N]>) {
    assert!(views
        .iter()
        .any(|view| view.iter().all(|val| *val != T::default())));
}

// TODO
pub fn assert_views_are_comparable<T: Debug + Default + PartialEq, const N: usize>(
    views: &Vec<[T; N]>,
) {
    for view1 in views {
        for view2 in views {
            for i in 0..N {
                if view1[i] != T::default() && view2[i] != T::default() {
                    assert_eq!(view1[i], view2[i])
                }
            }
        }
    }
}
