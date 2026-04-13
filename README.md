# 🐈 wool

[![tests](https://github.com/mentalblood0/wool/actions/workflows/tests.yml/badge.svg)](https://github.com/mentalblood0/wool/actions/workflows/tests.yml)

A Rust library for managing theses backed with [trove](https://github.com/mentalblood0/trove)

Each thesis is either text or relation between two existing theses

## Features

- tagging
- aliasing
- plain text commands processing
- graph generation

## Basic concepts

### Thesis

- optional **alias**
- **tag**s
- content: **text** or **relation**

**Thesis identifier** is 16 bytes fully determined by it's content (hash of text for text content and hash of binary representation of relation structure for relation content, hash function used is `xxhash128`) and represented in text and commands as url-safe non-padded base64 string, e.g. `ZqavF73LC9OQwCptOMUf1w`

#### Alias

Sequence of one or more non-whitespace characters, e.g. `(R-r).0`

#### Tag

Word characters sequence, e.g. `absolute_truth`

#### Text

- **raw text** with **references** inserted in it, e.g. `[(R-r).0] относительно истинно`

##### Reference

**Thesis identifier** or **alias** surrounded with square brackets, e.g. `[lvKjiQU1MkRfVFyJrWEaog]`, `[релятивизм]`

##### Raw text part

Cyrillic/Latin text: letters, whitespaces and punctuation marks `,-:.'"`

#### Relation

- **thesis identifier** from which it is
- **relation kind**
- **thesis identifier** to which it is

Supported relations kinds list is set in Sweater configuration file, e.g. see [`src/test_sweater_config.yml`](src/test_sweater_config.yml), so you can specify and use any relations kinds you like

##### Relation kind

An English words sequence without punctuation, e.g. `may be`, `therefore`

## Commands

If there is more then one command to parse, they must be delimited with two or more line breaks, e.g. see [`src/example.txt`](src/example.txt)

`/may Релятивизм опасен` - add **thesis** with **text** `Релятивизм опасен`

`/may R alias Общий релятивизм` - add **thesis**-**text** `Общий релятивизм` **alias**ed by `R`

`/may R-r includes (R-r).d` - add **thesis**-**relation** from `R-r` to `(R-r).d` by **relation kind** `includes`

`/may ((A1.1.2)/(R-r)).3.1 alias R includes A` - add **thesis**-**relation** from `R` to `A` by **relation kind** `includes` **alias**ed by `((A1.1.2)/(R-r)).3.1`

`/may total truth tag (R-r).0` - add tags `total` and `truth` to **thesis** with alias `(R-r).0`

`/may total truth not tag (R-r).0` - remove tags `total` and `truth` from **thesis** with alias `(R-r).0`

`/may (R-r).0 alias (R-r).0_lalala` - set **alias** `(R-r).0` for thesis with alias `(R-r).0_lalala`

Thesis can have no alias or one alias, so setting alias for already aliased thesis will replace it's alias. Internally theses are reference and relate to each other using theses identifiers, so replacing aliases won't break anything
