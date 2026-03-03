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

### Add text thesis

Two lines:

- `+` optionally followed by space and alias for this thesis 
- text

e.g.

```
+ (R-r).0_true_relatively
[(R-r).0] относительно истинно
```

### Add relation thesis

Four lines:

- `+` optionally followed by space and **alias** for this thesis 
- **thesis identifier** or **alias** of thesis *from* which this relation is
- **relation kind**
- **thesis identifier** or **alias** of thesis *to* which this relation is

e.g.

```
+ 
(R-r).d
therefore
(R-r).0
```

### Remove thesis

Two lines:

- `-`
- **thesis identifier** or **alias** of thesis to remove

e.g.

```
+ 
(R-r).d
```

Note that this will also remove all related and referencing theses

### Tag thesis

Three or more lines:

- `#`
- **thesis identifier** or **alias** of thesis to which add tags
- **tag** to add
- ...

e.g.

```
#
(R-r).0
total
truth
```

### Untag thesis

Three or more lines:

- `^`
- **thesis identifier** or **alias** of thesis from which remove tags
- **tag** to remove
- ...

e.g.

```
^
(R-r).0
total
truth
```

### Set alias

Two lines:

- `+` followed by space and **alias** to set for this thesis 
- **thesis identifier** or current **alias** of thesis for which to set alias from first line

Thesis can have no alias or one alias, so setting alias for already aliased thesis will replace it's alias. Internally theses are reference and relate to each other using theses identifiers, so replacing aliases won't break anything
