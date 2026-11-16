# Proposal

## Why

In the compact sidebar an agent row is its avatar and state dot alone, and its
tooltip gave only the agent's name. What the agent was doing, and why it was
waiting, was out of sight until the sidebar was widened (#253).

## What Changes

- A compact row's tooltip gives the agent's name, its state - worded as its
  status dot's tooltip, waiting reason and session limit included - and its
  activity line when it has one. A shell agent, which has no dot, shows no
  state.

## Impact

- `knot`: the compact row's tooltip (`sidebar_compact::compact_tooltip`).
