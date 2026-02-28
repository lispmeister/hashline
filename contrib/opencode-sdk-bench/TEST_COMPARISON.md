# Old vs New Test Suite Comparison

## Old Test Suite (fixtures-old/)

### Structure
- 3 sizes × 9 cases = **27 total tests**
- Cases: json, markdown_text, markdown_embedded_json, markdown_embedded_typescript, rust, rust_embedded_json, typescript, typescript_embedded_json, polyglot_complex

### Example Tasks (Too Easy!)
```
rust/task.md: "Fix should_log so it returns flag and not !flag"
typescript/task.md: "Fix isReady to return input and not !input"  
json/task.md: "Restore defaults.safeMode back to true"
markdown_text/task.md: "Restore final status line to PASS"
```

### Problems
- ❌ Trivial boolean negations (!flag → flag)
- ❌ No ambiguity or challenge
- ❌ Doesn't test precision (any targeting method works)
- ❌ Size variations don't add value (just repetition)
- ❌ Expensive: $0.127 per run

---

## New Test Suite (fixtures/)

### Structure  
- 1 size × 6 cases = **6 total tests**
- Every test is a unique challenge

### Example Tasks (Challenging!)
```
md-ambiguous-lines: "Change Mode: staging to Mode: production 
                     in Production section (not Development!)"

json-deep-nest: "Change replica.ssl from false to true
                (navigate to $.service.config.database.replica.ssl)"

rust-whitespace: "Re-indent line from 4 spaces to 8 spaces
                 (pure whitespace fix, no content change)"

ts-similar-names: "Modify UserValidator.validateUserEmail() class method
                  (not the standalone function with same name!)"

json-array-puzzle: "Fix 'build' step in 'deploy' stage
                   (not the 'build' stage itself!)"

md-json-embedded: "Change timeout in production JSON block
                  (not development block!)"
```

### Improvements
- ✅ Each test has a "gotcha" that catches sloppy targeting
- ✅ Tests precision: similar lines, deep nesting, scope awareness
- ✅ Realistic scenarios (config files, class vs function, arrays)
- ✅ Diverse challenges (whitespace, JSON paths, code fences)
- ✅ Efficient: $0.028 per run (78% cheaper!)

---

## Side-by-Side Example

### OLD: rust/small/mutated.rs
```rust
fn should_log(flag: bool) -> bool {
    !flag  // ← Just flip this to 'flag'
}
```
**Task**: "Fix should_log so it returns flag and not !flag"  
**Challenge Level**: 🟢 Trivial

### NEW: rust-whitespace/mutated.rs
```rust
pub fn set(&mut self, key: String, value: String) {
self.settings.insert(key, value);  // ← Wrong indent (4 spaces)
}
```
**Task**: "Re-indent to 8 spaces (preserve exact whitespace)"  
**Challenge Level**: 🔴 Hard (tests anchor precision + whitespace handling)

---

## Coverage Comparison

| Category | Old | New | Notes |
|----------|-----|-----|-------|
| Markdown | 9 tests | 2 tests | New ones test ambiguity + embedded JSON |
| JSON | 9 tests | 2 tests | New ones test deep nesting + arrays |
| Rust | 9 tests | 1 test | New one tests whitespace precision |
| TypeScript | 9 tests | 1 test | New one tests scope disambiguation |
| **Total** | **27** | **6** | 78% reduction |
| **Quality** | Low | High | Every test has a "trap" |
| **Cost/run** | $0.127 | $0.028 | 78% savings |

---

## Conclusion

The new suite is:
- **Harder** (each test catches real-world editing mistakes)
- **Smarter** (focuses on precision, not repetition)
- **Cheaper** (78% cost reduction)

Perfect for stress-testing hashline's anchor-based editing vs raw string replacement!
