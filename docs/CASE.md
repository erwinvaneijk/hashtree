# CASE (Cyber-investigation Analysis Standard Expression) Support

Hashtree now supports exporting file hash digests in **CASE JSON-LD format**, enabling forensically compliant output that can be integrated with other cyber-investigation tools.

## What is CASE?

**CASE** (Cyber-investigation Analysis Standard Expression) is an international standard developed by the Cyber Domain Ontology (CDO) community for representing cyber-investigation information. It provides:

- **Standardized ontology** for digital evidence representation
- **JSON-LD format** for machine-readable output
- **Traceability** of analysis results back to tools, performers, and timestamps
- **Interoperability** with forensic platforms and automated analysis tools

Learn more: https://caseontology.org/

## Usage

### Basic CASE Export

Export file hashes in CASE JSON-LD format:

```bash
hashtree create --sha-256 --style case -o output.jsonld files/
```

### With All Algorithms

```bash
hashtree create --all --style case -o evidence.jsonld ./evidence_files/
```

### Compact Format (no pretty-printing)

```bash
hashtree create --sha-256 --style case --compact-json -o hashes.jsonld files/
```

### Hash Values Are Hexadecimal

CASE types `types:hashValue` as `xsd:hexBinary`, so CASE manifests always hold
hexadecimal digests. `--base64` does not change a CASE manifest; it still
controls how digests are printed on the console. Manifests written by hashtree
3.0.1 and earlier may hold base64 digests; verification still accepts those.

## Output Structure

The output follows CASE/UCO 1.5.0. Every object has its own `urn:uuid:` IRI, as
UCO requires for facets and hashes. A manifest for one file looks like this
(see [examples/example_case_output.jsonld](../examples/example_case_output.jsonld)):

```json
{
  "@context": {
    "action": "https://ontology.unifiedcyberontology.org/uco/action/",
    "case-investigation": "https://ontology.caseontology.org/case/investigation/",
    "core": "https://ontology.unifiedcyberontology.org/uco/core/",
    "identity": "https://ontology.unifiedcyberontology.org/uco/identity/",
    "observable": "https://ontology.unifiedcyberontology.org/uco/observable/",
    "tool": "https://ontology.unifiedcyberontology.org/uco/tool/",
    "types": "https://ontology.unifiedcyberontology.org/uco/types/",
    "xsd": "http://www.w3.org/2001/XMLSchema#"
  },
  "@graph": [
    {
      "@id": "urn:uuid:face736b-7e6f-4bb8-b65d-31eb282004a1",
      "@type": "tool:AnalyticTool",
      "core:name": "hashtree",
      "tool:toolType": "hash-calculator",
      "tool:version": "4.0.0"
    },
    {
      "@id": "urn:uuid:3600e66d-eb63-4d89-ac54-381522d92bf6",
      "@type": "identity:Person",
      "core:name": "forensic_examiner"
    },
    {
      "@id": "urn:uuid:66f9b775-5ffa-46e1-81b6-f399ea32d6fd",
      "@type": "observable:File",
      "core:hasFacet": [
        {
          "@id": "urn:uuid:ed136e4f-70c1-4aa5-a7be-ad4f6e354622",
          "@type": "observable:FileFacet",
          "observable:fileName": "notes.txt",
          "observable:filePath": "evidence/notes.txt"
        },
        {
          "@id": "urn:uuid:17056f34-6993-4477-afcd-1e564b964255",
          "@type": "observable:ContentDataFacet",
          "observable:hash": [
            {
              "@id": "urn:uuid:555a2db2-0514-4aeb-950d-50415914e56c",
              "@type": "types:Hash",
              "types:hashMethod": "SHA256",
              "types:hashValue": {
                "@type": "xsd:hexBinary",
                "@value": "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
              }
            }
          ]
        }
      ]
    },
    {
      "@id": "urn:uuid:3825a8d5-94ad-44de-85c5-7848ddd81d8b",
      "@type": "case-investigation:InvestigativeAction",
      "core:name": "Compute digests",
      "action:startTime": {
        "@type": "xsd:dateTime",
        "@value": "2026-10-20T11:54:35.116+02:00"
      },
      "action:performer": { "@id": "urn:uuid:3600e66d-eb63-4d89-ac54-381522d92bf6" },
      "action:instrument": { "@id": "urn:uuid:face736b-7e6f-4bb8-b65d-31eb282004a1" },
      "action:result": [
        { "@id": "urn:uuid:66f9b775-5ffa-46e1-81b6-f399ea32d6fd" }
      ]
    }
  ]
}
```

## CASE Objects

The output includes these key CASE objects:

| Object | Purpose |
|--------|---------|
| `tool:AnalyticTool` | Documents the hashtree version |
| `identity:Person` | Records who performed the hashing |
| `observable:File` | One per hashed file, with its facets in `core:hasFacet` |
| `observable:FileFacet` | File path and name |
| `observable:ContentDataFacet` | The file's hashes in `observable:hash` |
| `types:Hash` | Algorithm (`types:hashMethod`) and hexadecimal digest (`types:hashValue`) |
| `case-investigation:InvestigativeAction` | When the hashing was done, by whom, with which tool, and which files resulted |

Hash methods use the names of the UCO `HashNameVocab`: `MD5`, `SHA1`, `SHA256`,
`SHA384`, `SHA512`, `SHA3-256`, `SHA3-384` and `SHA3-512`. RIPEMD-160 is not part
of that open vocabulary and is written as `RIPEMD160`.

## Validation

Validate a manifest against the CASE/UCO SHACL shapes with `case_validate` from
[case-utils](https://github.com/casework/CASE-Utilities-Python):

```bash
pip install case-utils
case_validate --built-version case-1.5.0 --allow-info output.jsonld
```

`--allow-info` is needed when the manifest holds RIPEMD-160 digests: the
validator reports `RIPEMD160` as an informational "not a member of the
vocabulary HashNameVocab" result.

To inspect the output with `jq`:

```bash
# Count files in output
jq '[.["@graph"][] | select(."@type" == "observable:File")] | length' output.jsonld

# Extract hash values
jq '.["@graph"][] | select(."@type" == "observable:File") | .["core:hasFacet"][]
    | select(."@type" == "observable:ContentDataFacet") | .["observable:hash"][]' output.jsonld
```

## Compatibility

- **CASE/UCO 1.5.0**: validated with `case_validate` (see above)
- **jHashtools**: writes and reads the same structure with `-s case`, so either
  tool can verify a manifest written by the other
- **Older manifests**: manifests written by hashtree 3.0.1 and earlier used other
  namespaces and kept the hashes directly on the file; `verify` recognises them
  and still reads them

## Verification

CASE format files can now be verified to detect tampering or changes:

```bash
# Create CASE manifest
hashtree create --sha-256 --style case -o hashes.jsonld files/

# Verify files against CASE manifest
hashtree verify --input hashes.jsonld --style case --sha-256 files/
```

### Verification Workflow

1. **Create manifest** with all 9 algorithms for maximum integrity verification:
   ```bash
   hashtree create --all --style case -o evidence.jsonld /evidence/
   ```

2. **Later, verify files** against the manifest:
   ```bash
   hashtree verify --input evidence.jsonld --style case --all /evidence/
   ```

3. **Detect tampering** - the verification will report any hash mismatches:
   ```
   *** PASSED VERIFICATION ***
   # OR
   There are differences.
   These entries are seen, but missing in the evidence.jsonld list
   file.txt
     sha-256  <new hash>
   These entries are in the evidence.jsonld list, but not in the directory
   file.txt
     sha-256  <original hash>
   ```

## Limitations

- **No RDF/XML**: JSON-LD format only (RDF/XML export could be added in future versions)

## Future Enhancements

Planned improvements for future releases:

- Extended provenance metadata
- Digital evidence chain tracking
- RDF/XML export format
- CASE update/amendment support

## Examples

### Export evidence directory

```bash
hashtree create --all --style case -o digital_evidence.jsonld /mnt/evidence/
```

### Export with verification workflow

```bash
# Create manifest with all algorithms
hashtree create --all --style case -o hashes.jsonld files/

# Later, verify against CASE manifest with all algorithms
hashtree verify --input hashes.jsonld --style case --all files/
```

### Detect file tampering

```bash
# Create manifest
hashtree create --sha-256 --style case -o evidence.jsonld /path/to/files/

# Modify a file
echo "tampered" > /path/to/files/important.txt

# Verify - will report the changes
hashtree verify --input evidence.jsonld --style case --sha-256 /path/to/files/
# Output shows:
# There are differences.
# These entries are seen, but missing in the evidence.jsonld list
# /path/to/files/important.txt
#     sha-256  <new hash>
# These entries are in the evidence.jsonld list, but not in the directory
# /path/to/files/important.txt
#     sha-256  <original hash>
```

### Integrate with analysis tools

```bash
# Export in CASE format for automated analysis
hashtree create --sha-256 --style case -o forensic_analysis.jsonld evidence/

# Process with CASE-aware tools (to be implemented by downstream)
python3 -c "import json; d=json.load(open('forensic_analysis.jsonld')); ..."
```

## See Also

- [CASE Ontology](https://ontology.caseontology.org/)
- [CDO Community](https://cyberdomainontology.org/)
- [DFRWS Standards](https://dfrws.org/)
- [JSON-LD Specification](https://www.w3.org/TR/json-ld11/)
