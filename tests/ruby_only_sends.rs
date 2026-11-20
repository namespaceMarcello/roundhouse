//! Sends the analyzer types for every target but only the Ruby family can
//! run: `Array#fetch`, `each_slice`, `each_cons`, `lazy`, an enumerator's
//! `with_object`, `Array[...]`, `Hash[...]`, `Array.try_convert` (a nested
//! splat), `include?` on a `Class` value,
//! Kernel's `Rational(...)` (what a rational literal ingests as) and
//! Kernel's `Integer(...)` and `Float(...)`.
//!
//! The other emitters render each as the method name on its receiver,
//! which nothing there defines, so every one refuses the construct by name
//! instead of exiting 0 with a call that cannot run. The Ruby family emits
//! them and builds (the runs are in `array_fetch_runs`,
//! `array_batches_typing`, `container_index_constructor`,
//! `include_query_receivers` and `rational_literal`).
//!
//! `<=>` is refused the same way, call and definition, on the targets whose
//! emit writes the Ruby operator as it is (Crystal's own `<=>` answers an
//! Int32 where the emit declares Integer as Int64). TypeScript renders the
//! numeric comparison, so it is not refused there.

#[path = "support/scratch.rs"]
mod scratch;
use std::process::Command;

const MODEL: &str = r#"class Article < ApplicationRecord
  #: (Array[Integer]) -> Integer
  def self.first_plus(nums)
    nums.fetch(0).succ
  end

  #: (Array[Integer]) -> Integer
  def self.slices(nums)
    nums.each_slice(2).to_a.length
  end

  #: (Array[Integer]) -> Integer
  def self.pairs(nums)
    nums.each_cons(2).to_a.length
  end

  #: (Array[Integer]) -> untyped
  def self.first_even(nums)
    nums.lazy.filter_map { |n| n * 10 if n.even? }.first
  end

  #: (Array[Integer]) -> Array[Integer]
  def self.memo(nums)
    nums.each_cons(2).with_object([]) { |pair, seen| seen << pair.first }
  end

  #: () -> Array[Integer]
  def self.built
    Array[1, 2]
  end

  #: () -> Hash[String, Integer]
  def self.table
    Hash[[["a", 1]]]
  end

  #: () -> Rational
  def self.third
    1/3r
  end

  #: (Class) -> bool
  def self.tagged?(klass) = klass.include?(Tagged)

  #: (String) -> Integer
  def self.count(text) = Integer(text)

  #: (String) -> Float
  def self.ratio(text) = Float(text)
end
"#;

const SERVICE: &str = r#"class RowUnpack
  def self.rest
    _head, (_month, *row) = [1, [2, 3]]
    row
  end
end
"#;

const REFUSED: [&str; 12] = [
    "Array#fetch",
    "Array#each_slice",
    "Array#each_cons",
    "Array#lazy",
    "Enumerator#with_object",
    "Array[]",
    "Hash[]",
    "Array.try_convert",
    "Kernel#Rational",
    "Module#include?",
    "Kernel#Integer",
    "Kernel#Float",
];

fn build(root: &std::path::Path, target: &str) -> (bool, String) {
    let out = root.join(format!("out-{target}"));
    let output = Command::new(env!("CARGO_BIN_EXE_roundhouse"))
        .args(["--target", target])
        .arg(root)
        .arg("-o")
        .arg(&out)
        .output()
        .unwrap();
    (output.status.success(), String::from_utf8_lossy(&output.stderr).into_owned())
}

#[test]
fn only_the_ruby_family_emits_the_ruby_only_sends() {
    let root = scratch::dir("rh_ruby_only_sends");
    for (path, text) in [
        ("db/schema.rb", "ActiveRecord::Schema.define do\n  create_table \"articles\" do |t|\n    t.string \"title\"\n  end\nend\n"),
        ("app/models/application_record.rb", "class ApplicationRecord < ActiveRecord::Base\n  self.abstract_class = true\nend\n"),
        ("app/models/tagged.rb", "module Tagged\n  def tagged = true\nend\n"),
        ("app/models/article.rb", MODEL),
        ("app/services/row_unpack.rb", SERVICE),
    ] {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
    for target in ["rust", "typescript", "go", "crystal", "python", "elixir"] {
        let (ok, text) = build(&root, target);
        assert!(!ok, "{target}: {text}");
        for construct in REFUSED {
            assert!(
                text.contains(&format!("error[unsupported]: {construct} not supported ({target})")),
                "{target} must refuse {construct}: {text}"
            );
        }
    }
    for target in ["ruby", "jruby", "spinel"] {
        let (ok, text) = build(&root, target);
        assert!(ok, "{target}: {text}");
        assert!(!text.contains("only the Ruby targets implement it"), "{target}: {text}");
    }
    std::fs::remove_dir_all(&root).unwrap();
}

/// An app method named like the Kernel conversion is the app's own, so a
/// call to it is not refused as Kernel's.
#[test]
fn an_app_method_named_integer_is_not_refused_as_kernel_s() {
    let root = scratch::dir("rh_ruby_only_sends_shadow");
    for (path, text) in [
        ("db/schema.rb", "ActiveRecord::Schema.define do\n  create_table \"articles\" do |t|\n    t.string \"title\"\n  end\nend\n"),
        ("app/models/application_record.rb", "class ApplicationRecord < ActiveRecord::Base\n  self.abstract_class = true\nend\n"),
        ("app/models/article.rb", "class Article < ApplicationRecord\n  #: (String) -> Integer\n  def Integer(text) = text.length\n\n  #: () -> Integer\n  def title_length = Integer(title.to_s)\nend\n"),
    ] {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
    let (_ok, text) = build(&root, "rust");
    assert!(!text.contains("Kernel#Integer not supported"), "{text}");
    std::fs::remove_dir_all(&root).unwrap();
}

const SPACESHIP: &str = r#"class Article < ApplicationRecord
  def <=>(other)
    id <=> other.id
  end

  #: (Integer, Integer) -> Integer
  def self.order(a, b)
    a <=> b
  end

  #: (Integer, Integer) -> Integer
  def self.reversed(a, b)
    (a <=> b) * -1
  end

  #: (Float, Float) -> Integer?
  def self.floats(a, b)
    a <=> b
  end

  #: (Symbol, Symbol) -> Integer
  def self.symbols(a, b)
    a <=> b
  end

  #: (Article, Article) -> Integer?
  def self.records(a, b)
    a <=> b
  end

  #: (Array[Integer]) -> Array[Integer]
  def self.sorted(xs)
    xs.sort { |x, y| x <=> y }
  end

  #: (Array[Integer]) -> Array[Integer]
  def self.sorted_after_a_step(xs)
    xs.sort do |x, y|
      _gap = x.to_s
      x <=> y
    end
  end

  #: (String, String) -> Integer
  def self.strings(a, b)
    a <=> b
  end
end
"#;

/// `<=>` refusals a target reports, by its own message: (calls, definitions).
fn spaceship_refusals(text: &str, target: &str) -> (usize, usize) {
    (
        text.matches(&format!("error[unsupported]: <=> not supported ({target})")).count(),
        text.matches(&format!("error[unsupported]: def <=> not supported ({target})")).count(),
    )
}

#[test]
fn the_comparison_operator_is_refused_where_the_emit_writes_it_as_it_is() {
    let root = scratch::dir("rh_ruby_only_sends_spaceship");
    for (path, text) in [
        ("db/schema.rb", "ActiveRecord::Schema.define do\n  create_table \"articles\" do |t|\n    t.string \"title\"\n  end\nend\n"),
        ("app/models/application_record.rb", "class ApplicationRecord < ActiveRecord::Base\n  self.abstract_class = true\nend\n"),
        ("app/models/article.rb", SPACESHIP),
    ] {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
    // Nine calls (Integer, reversed Integer, Float, Symbol, Article, the two sort blocks, String, and the one
    // inside `def <=>`) and the definition.
    for target in ["elixir", "go", "kotlin", "python", "rust", "swift", "csharp"] {
        let (ok, text) = build(&root, target);
        assert!(!ok, "{target}: {text}");
        assert_eq!(spaceship_refusals(&text, target), (9, 1), "{target}: {text}");
    }
    // Crystal's own `<=>` is right as the value of a sort block (its last statement), and nowhere else here.
    let (ok, text) = build(&root, "crystal");
    assert!(!ok, "crystal: {text}");
    assert_eq!(spaceship_refusals(&text, "crystal"), (7, 1), "crystal: {text}");
    // TypeScript renders the numbers and refuses String by its own table: here the Symbol, Article and the two
    // untyped sort-block receivers, and no definition (it drops `def <=>`).
    let (ok, text) = build(&root, "typescript");
    assert!(!ok, "typescript: {text}");
    assert_eq!(spaceship_refusals(&text, "typescript"), (4, 0), "typescript: {text}");
    assert!(text.contains("error[unsupported]: String#<=> not supported (typescript)"), "typescript: {text}");
    for target in ["ruby", "jruby", "spinel"] {
        let (ok, text) = build(&root, target);
        assert!(ok, "{target}: {text}");
        assert!(!text.contains("<=> not supported"), "{target}: {text}");
    }
    std::fs::remove_dir_all(&root).unwrap();
}
