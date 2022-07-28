# What

This crate contains a structure that acquires data form an indeterminately big
amount of files in a directory. It also provides Error types and builders for
the structure.

It assumes that a directory is a table with each file in the directory being an
entry. More information in the docs.

# Why

This crate exists in order to solve the following problem. You need your user
to hold to a bunch of data which they should be able to modify on their own. It
is simple structured data, of an undetermined length.

The problem with it is that a database is unreadable by the user and you need
to provide an interface to it. Should you want to provide said interface,
SQLite is ideal, with [several crates](https://lib.rs/search?q=sqlite) to
choose bindings from, being [rusqlite](https://lib.rs/crates/rusqlite) the most
popular one. If you want ORM (object relational mapping) there is
[diesel](https://diesel.rs) and [sea-QL](https://www.sea-ql.org/).

You don't want to provide the interface, or have to deal with SQL or with
database management. Shame on you, and shame on me for making it easier (I
hope) for you.

# [Dogfooding](https://en.wikipedia.org/wiki/Eating_your_own_dog_food)

I am personally using this crate in my application
[amisgitpm](https://github.com/david-soto-m/amisgitpm).
