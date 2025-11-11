# Requirements

1. Follow `serde_json` in API as close as possible
2. Focus on compatibility with the TOON spec, making sure ALL OF THE FIXTURES in https://github.com/toon-format/spec/tree/main/tests/fixtures are implemented and pass
3. Should have 100% test coverage
4. All operations should be reversible, i.e.: JSON -> TOON -> JSON; TOON -> JSON -> TOON for every example

## Implementation steps:

1. Research `serde` and `serde_json` APIs
   1. Find a common interface that would work for `serde_toon2`
   2. Verify that both serialization and deserialization are supported
   3. Produce a document: `API_RESEARCH.md`
2. [SPAWN AGENT]: Design the `serde_toon2` API, using `API_RESEARCH.md`
   - Produce: `API_DESIGN.md`
3. [SPAWN AGENT]: Using `API_DESIGN.md` as a guide, STUB OUT (no implementation) the external interface of `serde_toon2`
4. [SPAWN AGENT]: Build an extensive set of fixtures, rely on .json and .toon files and a test suite that iterates through all cases and runs them dyanmically, to make it easier to add and remove test files
   - Each test case should demonstrate that the fixture operations work in reverse order too
5. [SPAWN AGENT (parallel)]: using test fixtures and ./SPEC.md implement ONLY serialization in `serde_toon2`
6. [SPAWN AGENT (parallel)]: using test fixtures and ./SPEC.md implement ONLY deserialization in `serde_toon2`
7. Run tests to verify fixtures all pass
   - For each failure: [SPAWN AGENT]: implement a fix for test failure <failure text> <failure location>
