# MathCAT issue #827: pull request text

Written on Monday, September 28, 2026, by Agent W5c4, for the owner to file upstream from their own account when they choose (Wave 5 launch decisions). Nothing here has been sent anywhere. textweaver carries the same change in `third_party/mathcat` ([ADR-0036](../adr/0036-math-braille-and-navigation.md)).

## How to file it

1. Fork [daisy/MathCAT](https://github.com/daisy/MathCAT) and branch from `main`.
2. Apply the change to `src/interface.rs` below (it is also the second half of `third_party/mathcat/textweaver.patch`, made against the 0.7.6-rc.3 package; check that `main` has not moved those lines). Leave out the patch's `Cargo.toml` part: it only removes test targets textweaver does not vendor.
3. Add the test file below as `tests/navigation_braille.rs`, and a `[[test]]` entry for it if `Cargo.toml` lists tests by hand (0.7.6-rc.3 does).
4. Run `cargo test --features no-unsafe` and `cargo test`, then open the pull request with the title and description below.

Before filing, look at the issue again: if someone has linked a fix in the meantime, comment there with the test instead.

## Title

Fix GetNavigationBraille panic in no-unsafe builds (#827)

## Description

Fixes #827.

`get_navigation_braille` wraps the navigation node in a new `<math>` element in a temporary `Package` when the node is not the whole expression. It calls `copy_mathml(found)`, which allocates the copy in `found`'s own document, and then appends that copy to an element of the new document.

The unsafe sxd-document backend tolerates a node from another document. The no-unsafe backend does not: its nodes are indices into their own document's storage, so the appended subtree is read with the wrong document's indices, and the call panics with "index out of bounds" (caught by MathCAT's guard and returned as a "MathCAT crash" error). Any navigation step below the top level followed by `get_navigation_braille` triggers it, for example zooming into a fraction.

This change adds `copy_mathml_to(mathml, doc)`, which allocates every node of the copy in the given document, and uses it in `get_navigation_braille`. `copy_mathml` keeps its behavior (a copy in the element's own document), now written on the same recursive helper, so `navigate.rs` and other callers are unchanged. The other branch of `get_navigation_braille` (an offset into a token) already creates its elements in the new document.

Tested with `--features no-unsafe` and with the default features:

- the new test `tests/navigation_braille.rs` fails before the change (a "MathCAT crash" error on the step into the fraction) and passes after it;
- the existing tests pass.

The same change has been carried by textweaver, an open-source document reader that builds MathCAT 0.7.6-rc.3 with `no-unsafe`, where a regression test walks a fraction in Nemeth and UEB (zoom in, next, zoom out, start, end) and checks the braille at each step.

## The change to `src/interface.rs`

```diff
@@ -433,7 +433,9 @@
                                 Ok(found)
                             } else {
                                 let new_mathml = create_mathml_element(&new_doc, "math");
-                                new_mathml.append_child(copy_mathml(found));
+                                // Allocate the copy in `new_doc`: with the no-unsafe backend a node
+                                // can't be appended to another document's element (issue #827)
+                                new_mathml.append_child(copy_mathml_to(found, &new_doc));
                                 new_doc.root().append_child(new_mathml);
                                 Ok(new_mathml)
                             }
@@ -688,19 +690,28 @@
 /// The Element type does not copy and modifying the structure of an element's child will modify the element, so we need a copy
 /// Convert the returned error from set_mathml, etc., to a useful string for display
 pub fn copy_mathml(mathml: Element) -> Element {
-    return copy_mathml_recursive(mathml, 0);
+    let doc = mathml.document();
+    return copy_mathml_recursive(mathml, &doc, 0);
 }
 
-fn copy_mathml_recursive(mathml: Element, depth: usize) -> Element {
+/// Copy (recursively) the (MathML) element into `doc` and return the new one.
+/// Every node of the copy is allocated in `doc`, so the copy can be appended to an element of `doc`
+/// even when `mathml` belongs to another document. The no-unsafe backend requires this:
+/// its nodes are indices into their own document's storage (issue #827).
+pub fn copy_mathml_to<'d>(mathml: Element, doc: &Document<'d>) -> Element<'d> {
+    return copy_mathml_recursive(mathml, doc, 0);
+}
+
+fn copy_mathml_recursive<'d>(mathml: Element, doc: &Document<'d>, depth: usize) -> Element<'d> {
     // Safety: Prevent stack overflow on deeply nested MathML
     if depth > MAX_DEPTH {
         // Return the element as a leaf if it's too deep to prevent crash
-        return create_mathml_element(&mathml.document(), as_str!(name(mathml)));
+        return create_mathml_element(doc, as_str!(name(mathml)));
     }
 
     // If it represents MathML, the 'Element' can only have Text and Element children along with attributes
     let children = mathml.children();
-    let new_mathml = create_mathml_element(&mathml.document(), as_str!(name(mathml)));
+    let new_mathml = create_mathml_element(doc, as_str!(name(mathml)));
     mathml.attributes().iter().for_each(|attr| {
         new_mathml.set_attribute_value(as_qname!(attr.name()), as_str!(attr.value()));
     });
@@ -715,7 +726,7 @@
     let mut new_children = Vec::with_capacity(children.len());
     for child in children {
         let child = as_element(child);
-        let new_child = copy_mathml_recursive(child, depth + 1);
+        let new_child = copy_mathml_recursive(child, doc, depth + 1);
         new_children.push(new_child);
     }
     new_mathml.append_children(new_children);
```

## The test, `tests/navigation_braille.rs`

```rust
//! Issue #827: get_navigation_braille for a part of the expression, which
//! wraps a copy of the part in a new document. Run with and without
//! `--features no-unsafe`.
use libmathcat::interface::*;

const FRACTION: &str =
    "<math><mfrac><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow><mn>2</mn></mfrac></math>";

#[test]
fn navigation_braille_of_a_part() -> anyhow::Result<()> {
    set_rules_dir(libmathcat::abs_rules_dir_path())?;
    set_preference("BrailleCode", "Nemeth")?;
    set_preference("BrailleNavHighlight", "Off")?;
    set_mathml(FRACTION)?;
    do_navigate_command("ReadCurrent")?;
    // Into the fraction: the numerator is not the whole expression.
    do_navigate_command("ZoomIn")?;
    assert_eq!(get_navigation_braille()?, "⠁⠬⠃");
    do_navigate_command("MoveNext")?;
    assert_eq!(get_navigation_braille()?, "⠼⠆");
    Ok(())
}
```

The expected braille is what the same steps give in textweaver's regression test through the patched 0.7.6-rc.3 (the numerator "a + b", and the denominator "2" with its numeric indicator). The test file itself has not been run inside MathCAT's repository; run it there before filing, and adjust the rules-directory line if MathCAT's own tests set it up another way (see `tests/common` in the repository).

## See also

- [Issue #827](https://github.com/daisy/MathCAT/issues/827)
- [ADR-0036: Math braille and navigation](../adr/0036-math-braille-and-navigation.md)
- [The vendored copy](../../third_party/mathcat/TEXTWEAVER.md)
- [Documentation index](../README.md)
