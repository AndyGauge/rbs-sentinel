# What is RBS?

[RBS](https://github.com/ruby/rbs) ("Ruby Signature") is Ruby's own type signature language, maintained by ruby/ruby core. It describes the shape of your code — method signatures, class hierarchies, module mixins, constants — in separate `.rbs` files, without touching the Ruby source itself.

A signature for a plain method looks like this:

```rbs
class UserSerializer
  def self.call: (User) -> Hash[Symbol, untyped]
end
```

That's it: no implementation, just the type. `.rbs` files normally live in a `sig/` directory that mirrors your `app/`/`lib/` layout.

## Why signatures live in separate files

Unlike TypeScript-in-JavaScript or type-hinted Python, RBS types are **not inline in the `.rb` source** by default. This is a deliberate design choice by the RBS authors: it keeps the type layer fully optional and lets tooling (type checkers, IDEs) consume `.rbs` without parsing Ruby's much larger grammar.

The tradeoff is the one every RBS user runs into immediately: **the signature and the implementation can drift.** You rename a method, add a parameter, change a return type — and nothing forces the `.rbs` file to follow along. A stale signature is worse than no signature: it lies to your type checker with confidence.

## What actually checks these signatures

RBS itself is just the language and a parser/loader (the `rbs` gem). It doesn't check anything on its own. [Steep](./adding-steep.md) is the type checker built on top of it — it reads your `.rbs` files, reads your `.rb` implementation, and reports where they disagree.

So the practical pipeline is:

```text
.rb source  →  .rbs signatures  →  Steep  →  type errors
```

Sentinel exists entirely to make that first arrow (`.rb` → `.rbs`) automatic, so the signatures Steep checks against are never stale. See [What is inline RBS?](./inline-rbs.md) for how.
