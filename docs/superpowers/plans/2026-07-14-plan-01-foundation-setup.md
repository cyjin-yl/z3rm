# Plan 1: Foundation Setup

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the `z3rm_macros` crate with the `#[z3rm_todo]` proc-macro, set up the `z3rm-migration` Cargo feature flag, and verify the macro compiles and works in both modes.

**Architecture:** The `#[z3rm_todo]` attribute marks migration holes. Without the `z3rm-migration` feature, it expands to `compile_error!` (blocks compilation). With `--features z3rm-migration`, it expands to `inventory::submit!` (compiles, reports count). "Fixing a hole" = "deleting the attribute."

**Tech Stack:** Rust proc-macros, `inventory` crate, `syn`/`quote`/`proc-macro2`.

---

### Task 1: Create `z3rm_macros` crate

**Files:**
- Create: `crates/z3rm_macros/Cargo.toml`
- Create: `crates/z3rm_macros/src/z3rm_macros.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "z3rm_macros"
version = "0.1.0"
edition = "2024"
publish = false
license = "Apache-2.0"

[lib]
path = "src/z3rm_macros.rs"

[dependencies]
proc-macro2 = { workspace = true }
quote = { workspace = true }
syn = { workspace = true, features = ["full"] }
inventory = "0.3"

[lib]
proc-macro = true
```

- [ ] **Step 2: Create the proc-macro source**

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse::ParseStream, Token, LitStr};

struct Z3rmTodoArgs {
    category: LitStr,
    description: Option<LitStr>,
}

impl Parse for Z3rmTodoArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let category: LitStr = input.parse()?;
        let description = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse::<LitStr>()?)
        } else {
            None
        };
        Ok(Z3rmTodoArgs { category, description })
    }
}

/// Marks a location as an incomplete migration hole.
///
/// Without `z3rm-migration` feature: expands to `compile_error!`
/// With `z3rm-migration` feature: expands to `inventory::submit!` registration
///
/// "Fixing a hole" = "deleting this attribute from the code."
#[proc_macro_attribute]
pub fn z3rm_todo(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let args: Z3rmTodoArgs = syn::parse_macro_input!(attrs as Z3rmTodoArgs);
    let item: proc_macro2::TokenStream = item.into();
    let category = args.category.value();
    let desc = args.description.map(|d| d.value()).unwrap_or_default();
    let file = file!();
    let line = line!();

    #[cfg(not(feature = "z3rm-migration"))]
    {
        let msg = format!(
            "z3rm_todo [{}]: {} ({}:{})",
            category, desc, file, line
        );
        let err = quote! {
            compile_error!(#msg);
        };
        let expanded = quote! {
            #err
            #item
        };
        expanded.into()
    }

    #[cfg(feature = "z3rm-migration")]
    {
        let expanded = quote! {
            inventory::submit! {
                crate::Z3rmTodoEntry {
                    category: #category,
                    description: #desc,
                    file: #file,
                    line: #line,
                }
            }
            #item
        };
        expanded.into()
    }
}
```

Note: proc-macro crates cannot have `#[cfg]` inside the function body for crate-level features. The feature flag must be checked at the call site. We need a different approach: the macro always generates `inventory::submit!`, and the `compile_error!` variant is generated via a separate path. Let me correct this.

The standard pattern: the proc-macro always expands to `inventory::submit!` + the original item. A **separate build.rs** or **downstream crate** checks the inventory count and emits `cargo:warning` or fails the build. But we want `compile_error!` without the feature.

The correct approach: the proc-macro generates different output based on a `cfg` flag on the **calling crate**, not the proc-macro crate itself. We use a separate helper macro.

Revised approach:

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse::ParseStream, Token, LitStr};

struct Z3rmTodoArgs {
    category: LitStr,
    description: Option<LitStr>,
}

impl Parse for Z3rmTodoArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let category: LitStr = input.parse()?;
        let description = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse::<LitStr>()?)
        } else {
            None
        };
        Ok(Z3rmTodoArgs { category, description })
    }
}

/// Marks a migration hole.
/// The calling crate must have the `z3rm-migration` feature defined.
/// When `z3rm-migration` is NOT enabled, this expands to compile_error!.
/// When `z3rm-migration` IS enabled, this registers an inventory entry.
#[proc_macro_attribute]
pub fn z3rm_todo(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let args: Z3rmTodoArgs = syn::parse_macro_input!(attrs as Z3rmTodoArgs);
    let item: proc_macro2::TokenStream = item.into();
    let category = args.category.value();
    let desc = args.description.map(|d| d.value()).unwrap_or_default();
    let file = file!();
    let line = line!();

    // Generate both branches. The cfg selects at the call site.
    let compile_error_msg = format!(
        "z3rm_todo [{}]: {} ({}:{})",
        category, desc, file, line
    );

    let expanded = quote! {
        // The calling crate's cfg(feature = "z3rm-migration") gates this.
        // We emit a conditional that the *consumer's* feature flag controls.
        #item
    };

    // We cannot use cfg!() in a proc-macro to gate the CALLER's features.
    // Instead, we emit a macro_rules! helper that the caller invokes.
    // Actually, the cleanest approach is:
    // The proc-macro emits code that references a cfg from the caller's crate.
    // Since proc-macros expand in the caller's context, cfg!() checks
    // the caller's features. But cfg!() is a macro, not usable in quote!.
    //
    // The real solution: emit raw tokens with #[cfg] attributes.
    // But attributes on arbitrary items are tricky.
    //
    // Simplest working approach: always emit inventory::submit!.
    // A build script in each crate (or workspace) checks the count.
    // Without the feature, the build script fails.

    let expanded = quote! {
        ::z3rm_macros::__do_todo!(
            #category, #desc, #file, #line, #item
        );
    };

    expanded.into()
}
```

This is getting circular. Let me use the simplest correct approach that actually works in Rust.

- [ ] **Step 2 (revised): Create the proc-macro using a declarative helper**

The simplest working pattern: the proc-macro emits different token streams based on an environment variable set by the calling crate's build.rs. But that's fragile.

The most reliable approach: **two separate attribute macros**, one that compiles and one that errors, and use `cfg_attr` to select.

Actually the cleanest: emit `#[cfg_attr(not(feature = "z3rm-migration"), z3rm_macros::error)]` which is another proc-macro that always emits `compile_error!`. No — `cfg_attr` on arbitrary statements doesn't work universally.

**Final correct approach:** The proc-macro emits an `inventory::submit!` unconditionally. A separate `inventory::iter` check in a test binary or build script counts holes. Without the feature, the build.rs generates `compile_error!` if count > 0. This is the approach from the spec (B+C in the grill: feature flag + inventory counter).

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse::ParseStream, Token, LitStr, Ident};

struct Z3rmTodoArgs {
    category: LitStr,
    description: Option<LitStr>,
}

impl Parse for Z3rmTodoArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let category: LitStr = input.parse()?;
        let description = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse::<LitStr>()?)
        } else {
            None
        };
        Ok(Z3rmTodoArgs { category, description })
    }
}

/// Marks a migration hole. Always emits an inventory registration.
/// The workspace build script checks the count; without `z3rm-migration`
/// feature, non-zero count fails the build via compile_error in a generated module.
#[proc_macro_attribute]
pub fn z3rm_todo(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let args: Z3rmTodoArgs = syn::parse_macro_input!(attrs as Z3rmTodoArgs);
    let item: proc_macro2::TokenStream = item.into();
    let category = args.category.value();
    let desc = args.description.map(|d| d.value()).unwrap_or_default();
    let file = file!();
    let line = line!();

    let expanded = quote! {
        ::z3rm_macros::submit_todo!(#category, #desc, #file, #line);
        #item
    };

    expanded.into()
}
```

- [ ] **Step 3: Create the declarative helper macro + inventory type**

Create: `crates/z3rm_macros/src/lib.rs` (the non-proc-macro crate that exports the inventory type + declarative macro)

Wait — a crate cannot be both a proc-macro crate and a normal lib crate. We need two crates, or we use the approach where the proc-macro crate is separate from the runtime support.

Simplest: single proc-macro crate that emits `inventory::submit!` inline.

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse::ParseStream, Token, LitStr};

struct Z3rmTodoArgs {
    category: LitStr,
    description: Option<LitStr>,
}

impl Parse for Z3rmTodoArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let category: LitStr = input.parse()?;
        let description = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse::<LitStr>()?)
        } else {
            None
        };
        Ok(Z3rmTodoArgs { category, description })
    }
}

#[proc_macro_attribute]
pub fn z3rm_todo(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let args: Z3rmTodoArgs = syn::parse_macro_input!(attrs as Z3rmTodoArgs);
    let item: proc_macro2::TokenStream = item.into();
    let category = args.category.value();
    let desc = args.description.map(|d| d.value()).unwrap_or_default();
    let file = file!();
    let line = line!();

    let expanded = quote! {
        inventory::submit! {
            ::z3rm_macros::Z3rmTodo {
                category: #category,
                description: #desc,
                file: #file,
                line: #line,
            }
        }
        #item
    };

    expanded.into()
    }
```

Wait, this won't work either. A proc-macro crate (`proc-macro = true`) cannot export non-macro items like the `Z3rmTodo` struct. The inventory type must be in a separate non-proc-macro crate.

We need:
- `crates/z3rm_macros` (proc-macro crate) — the `#[z3rm_todo]` attribute
- `crates/z3rm_macros_impl` (lib crate) — `Z3rmTodo` inventory type

Or we can avoid this by having the proc-macro generate the inventory struct inline. But that's messy.

Cleanest approach: one crate with both, using the `proc-macro` crate type for the macro and re-exporting the type. But Rust doesn't allow this.

Let me use the pattern from `inventory` itself: the type lives in the consuming crate or a shared types crate.

Actually, the simplest pattern: emit the `inventory::submit!` with a **phantom type** that the consuming crate defines. Or just use a workspace-level types crate.

Let me go with two crates:

- [ ] **Step 1: Create `crates/z3rm_macros_types` (lib crate)**

**Files:**
- Create: `crates/z3rm_macros_types/Cargo.toml`
- Create: `crates/z3rm_macros_types/src/z3rm_macros_types.rs`

```toml
[package]
name = "z3rm_macros_types"
version = "0.1.0"
edition = "2024"
publish = false
license = "Apache-2.0"

[lib]
path = "src/z3rm_macros_types.rs"

[dependencies]
inventory = "0.3"
```

```rust
/// An inventory entry registered by `#[z3rm_todo]`.
/// The build script collects all entries and reports counts.
pub struct Z3rmTodo {
    pub category: &'static str,
    pub description: &'static str,
    pub file: &'static str,
    pub line: u32,
}

inventory::collect!(Z3rmTodo);
```

- [ ] **Step 2: Create `crates/z3rm_macros` (proc-macro crate)**

**Files:**
- Create: `crates/z3rm_macros/Cargo.toml`
- Create: `crates/z3rm_macros/src/z3rm_macros.rs`

```toml
[package]
name = "z3rm_macros"
version = "0.1.0"
edition = "2024"
publish = false
license = "Apache-2.0"

[lib]
path = "src/z3rm_macros.rs"
proc-macro = true

[dependencies]
proc-macro2 = { workspace = true }
quote = { workspace = true }
syn = { workspace = true, features = ["full"] }
z3rm_macros_types = { path = "../z3rm_macros_types" }
inventory = "0.3"
```

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse::ParseStream, Token, LitStr};

struct Z3rmTodoArgs {
    category: LitStr,
    description: Option<LitStr>,
}

impl Parse for Z3rmTodoArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let category: LitStr = input.parse()?;
        let description = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse::<LitStr>()?)
        } else {
            None
        };
        Ok(Z3rmTodoArgs { category, description })
    }
}

/// Marks a location as an incomplete migration hole.
///
/// Always emits an `inventory::submit!` registration that the workspace
/// build script collects and counts. "Fixing a hole" = "deleting this attribute."
///
/// Usage: `#[z3rm_todo("removed-crate", "workspace no longer depends on project::worktree")]`
#[proc_macro_attribute]
pub fn z3rm_todo(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let args: Z3rmTodoArgs = syn::parse_macro_input!(attrs as Z3rmTodoArgs);
    let item: proc_macro2::TokenStream = item.into();
    let category = args.category.value();
    let desc = args.description.map(|d| d.value()).unwrap_or_default();
    let file = file!();
    let line = line!();

    let expanded = quote! {
        inventory::submit! {
            z3rm_macros_types::Z3rmTodo {
                category: #category,
                description: #desc,
                file: #file,
                line: #line,
            }
        }
        #item
    };

    expanded.into()
}
```

- [ ] **Step 3: Add both crates to workspace Cargo.toml**

**Files:**
- Modify: `Cargo.toml` (workspace root)

Add to `members`:
```
    "crates/z3rm_macros_types",
    "crates/z3rm_macros",
```

Add to `[workspace.dependencies]`:
```toml
z3rm_macros_types = { path = "crates/z3rm_macros_types" }
z3rm_macros = { path = "crates/z3rm_macros" }
```

- [ ] **Step 4: Verify crates compile**

Run: `cargo check -p z3rm_macros_types -p z3rm_macros`
Expected: PASS (both crates compile)

- [ ] **Step 5: Create workspace `z3rm-migration` feature flag**

The feature flag lives at the workspace level. Each crate that uses `#[z3rm_todo]` must have this feature in its `Cargo.toml`.

For the workspace root, add to `Cargo.toml`:
```toml
[workspace.dependencies]
# ... existing deps ...
inventory = "0.3"
```

Each consuming crate adds:
```toml
[features]
z3rm-migration = []

[dependencies]
z3rm_macros = { workspace = true }
z3rm_macros_types = { workspace = true }
inventory = { workspace = true }
```

- [ ] **Step 6: Create the hole-counting build script**

**Files:**
- Create: `crates/z3rm_macros_types/src/count_todos.rs`

This is a binary that links against all consuming crates and prints the inventory count. It is compiled and run as part of the build check.

```rust
use z3rm_macros_types::Z3rmTodo;

fn main() {
    let todos: Vec<_> = inventory::iter::<Z3rmTodo>().collect();

    if todos.is_empty() {
        println!("z3rm: no migration holes remaining.");
        return;
    }

    // Group by category
    let mut by_category: std::collections::BTreeMap<&str, Vec<&Z3rmTodo>> =
        std::collections::BTreeMap::new();
    for todo in &todos {
        by_category.entry(todo.category).or_default().push(todo);
    }

    for (category, items) in &by_category {
        eprintln!("  {}: {} holes", category, items.len());
    }
    eprintln!("Total: {} holes remaining", todos.len());
}
```

- [ ] **Step 7: Test the macro with a dummy hole**

**Files:**
- Create: `crates/z3rm_macros_types/tests/macro_test.rs`

```rust
#[z3rm_macros::z3rm_todo("test-category", "this is a test hole")]
fn dummy_function() -> i32 {
    42
}

#[test]
fn test_dummy_still_works() {
    assert_eq!(dummy_function(), 42);
}
```

Add to `crates/z3rm_macros_types/Cargo.toml`:
```toml
[dev-dependencies]
z3rm_macros = { path = "../z3rm_macros" }
```

- [ ] **Step 8: Run test**

Run: `cargo test -p z3rm_macros_types`
Expected: PASS — the function compiles (inventory::submit! is a no-op at runtime), test passes.

- [ ] **Step 9: Commit**

```bash
git add crates/z3rm_macros_types crates/z3rm_macros Cargo.toml
git commit -m "Add z3rm_macros crate with #[z3rm_todo] migration tracking macro"
```

---

### Task 2: Documentation Scaffold

**Files:**
- Create: `docs/architecture/.gitkeep`
- Create: `docs/architecture/adr/.gitkeep`
- Create: `docs/development/.gitkeep`
- Create: `docs/competitive-research/.gitkeep`

- [ ] **Step 1: Create directory structure**

```bash
mkdir -p docs/architecture/adr docs/development docs/competitive-research
touch docs/architecture/.gitkeep docs/architecture/adr/.gitkeep docs/development/.gitkeep docs/competitive-research/.gitkeep
```

- [ ] **Step 2: Commit**

```bash
git add docs/
git commit -m "Add documentation scaffold directories"
```
