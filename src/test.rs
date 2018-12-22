use std::cell::RefCell;
use std::rc::Rc;

enum A {
  Node(Rc<RefCell<A>>),
  End,
}

fn main() -> () {
  let a = Rc::new(RefCell::new(A::Node(
    Rc::new(RefCell::new(A::Node(
      Rc::new(RefCell::new(A::End))))))));

  let mut pointer = &a;
  let mut count = 0;

  while let A::Node(ref v) = *pointer.borrow() {
    count += 1;
    pointer = v;
  }

  println!("{} elements", count);
}
