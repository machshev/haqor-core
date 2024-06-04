# 2. use nix develop shell for consistent dev environment

Date: 2024-06-04

## Status

Accepted



## Context

When switching between projects it's a challenge to make sure the development
environment is set up as required for each project. We want to make it easy for
anyone to quickly set up a development environment and focus on the development
itself and not on seting up toolchains.

In the context of this ADR we are defining development enviroment as the
toolchains and the dependencies, rather than the IDE and configuration of the
IDE itself. We asume that each developer has their own editor configuration as
they like it and so this is explicitly out of scope.

## Decision

Provide flake based nix development shell with a working set of dependancies.
NixOS is not required to use this, nix can be used on your favourite distro.
It's not required to use the nix devshell, but should simplify the setup for
those who want to use it.

## Consequences

This makes it a lot easier to get started with development if you have nix
available.

One potential downside is that the documentation may need to be duplicated in
places for nix and non-nix setups. If this becomes an issue later then it should
be addressed then.
