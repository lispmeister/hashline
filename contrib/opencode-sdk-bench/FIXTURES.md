# Minimal Comprehensive Test Suite

This test suite replaces 27 tests (9 cases × 3 sizes) with **6 carefully designed challenges** that stress-test editing capabilities while minimizing API costs.

## Design Principles

1. **Ambiguity** - Multiple similar lines/keys to test precise targeting
2. **Nesting** - Deep structures requiring careful navigation
3. **Whitespace** - Indentation-sensitive fixes
4. **Context** - Similar names in different scopes
5. **Multi-format** - Embedded code blocks

## Test Cases

### 1. md-ambiguous-lines (Markdown)
**File**: `original.md` (24 lines)
**Challenge**: Multiple lines containing "Mode:" - agent must use context to identify correct line
**Task**: Change `Mode: staging` to `Mode: production` in the Production Mode section (not Development!)
**What it tests**: Line disambiguation using section headers as anchors

### 2. json-deep-nest (JSON)
**File**: `original.json` (30 lines)
**Challenge**: Deeply nested structure with duplicate keys (`ssl` appears twice)
**Task**: Change `$.service.config.database.replica.ssl` from `false` to `true`
**What it tests**: JSONPath navigation, handling duplicate keys at different levels

### 3. rust-whitespace (Rust)
**File**: `original.rs` (35 lines)  
**Challenge**: Purely whitespace-based fix (no content change)
**Task**: Re-indent line from 4 spaces to 8 spaces in `set` method
**What it tests**: Exact whitespace preservation, indentation handling

### 4. ts-similar-names (TypeScript)
**File**: `original.ts` (36 lines)
**Challenge**: Multiple functions with nearly identical names and signatures
**Task**: Modify `UserValidator.validateUserEmail()` class method, not the standalone function
**What it tests**: Scope awareness, distinguishing class methods from functions

### 5. json-array-puzzle (JSON)
**File**: `original.json` (27 lines)
**Challenge**: Arrays with objects having duplicate "name" fields
**Task**: Find "build" step inside "deploy" stage (not the "build" stage itself!)
**What it tests**: Array indexing, nested object access, name collision handling

### 6. md-json-embedded (Markdown + JSON)
**File**: `original.md` (28 lines)
**Challenge**: Markdown file containing multiple JSON code blocks
**Task**: Change timeout in **production** JSON block from 3000 to 10000 (not development!)
**What it tests**: Multi-format editing, code fence awareness, value disambiguation

## Coverage Summary

| Format | Tests | Key Skills Tested |
|--------|-------|-------------------|
| Markdown | 2 | Context-based targeting, embedded JSON |
| JSON | 2 | Deep nesting, arrays, duplicate keys |
| Rust | 1 | Whitespace precision |
| TypeScript | 1 | Scope disambiguation |

## Cost Impact

**Old setup**: 27 test cases per run  
**New setup**: 6 test cases per run  
**Savings**: 78% reduction in API calls

**Estimated cost per run** (using gpt-5.3-codex @ $0.0047/attempt):
- Old: 27 × $0.0047 = **$0.127**
- New: 6 × $0.0047 = **$0.028**
- **Savings**: $0.099 per run (78% cheaper)

## Why This Works

Each test is a **surgical puzzle** designed to expose common editing failures:

1. **Anchoring mistakes** - Editing wrong instance of similar text
2. **Navigation errors** - Getting lost in deep structures  
3. **Whitespace bugs** - Breaking indentation
4. **Scope confusion** - Mixing up standalone vs class members
5. **Format blindness** - Missing code fence boundaries

These 6 tests catch **more bugs per dollar** than 27 simple boolean flips.
