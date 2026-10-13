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

## Output Structure

CASE output follows the JSON-LD format with `@context` and `@graph` fields:

```json
{
  "@context": {
    "rdf": "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "observable": "https://ontology.caseontology.org/observable/",
    "types": "https://ontology.caseontology.org/types/",
    "investigation": "https://ontology.caseontology.org/investigation/",
    "action": "https://ontology.caseontology.org/action/",
    "tool": "https://ontology.caseontology.org/tool/",
    "core": "https://ontology.caseontology.org/core/",
    "identity": "https://ontology.caseontology.org/identity/"
  },
  "@graph": [
    // Tool description
    {
      "@id": "case:tool-hashtree-1",
      "@type": "tool:AnalyticTool",
      "tool:name": "hashtree",
      "tool:version": "1.0.0",
      "tool:toolType": "hash-calculator"
    },
    // Operator/performer
    {
      "@id": "case:identity-operator",
      "@type": "identity:Person",
      "core:name": "username"
    },
    // File and hashes
    {
      "@id": "case:file-1",
      "@type": "observable:File",
      "observable:filePath": "/path/to/file.txt",
      "observable:fileName": "file.txt",
      "observable:hashes": [
        {
          "types:hashMethod": "SHA-256",
          "types:hashValue": "1f11452d48f3435b4447347b259055774020dab4e936b8c4c1f1b5ecd05a8ed1"
        }
      ]
    },
    // Investigative action
    {
      "@id": "case:action-hash-1",
      "@type": "investigation:InvestigativeAction",
      "investigation:performer": "case:identity-operator",
      "action:startTime": "2026-10-12T10:08:06.020+02:00",
      "action:instrument": { "@id": "case:tool-hashtree-1" },
      "action:result": { "@id": "case:bundle-results" }
    },
    // Results bundle
    {
      "@id": "case:bundle-results",
      "@type": "core:Bundle",
      "core:hasFacet": [
        { "@id": "case:file-1" }
      ]
    }
  ]
}
```

## CASE Objects

The output includes these key CASE objects:

| Object | Type | Purpose |
|--------|------|---------|
| `tool:AnalyticTool` | Tool description | Documents hashtree version and capabilities |
| `identity:Person` | Operator info | Records who performed the hashing |
| `observable:File` | File metadata | Contains file path and hashes |
| `types:Hash` | Hash values | Algorithm and hexadecimal digest |
| `investigation:InvestigativeAction` | Action metadata | Documents when and how hashing was performed |
| `core:Bundle` | Results wrapper | Groups all objects together |

## Validation

To validate CASE output structure:

```bash
# Check if output is valid JSON
jq . output.jsonld

# Verify presence of required fields
jq '.["@context"]' output.jsonld
jq '.["@graph"][] | select(."@type" == "observable:File")' output.jsonld

# Count files in output
jq '.["@graph"][] | select(."@type" == "observable:File")' output.jsonld | wc -l

# Extract hash values
jq '.["@graph"][] | select(."@type" == "observable:File") | .["observable:hashes"][]' output.jsonld
```

## Compatibility

- **JSON-LD**: Full compliance with JSON-LD specification
- **CASE Ontology**: Uses official CASE ontology namespaces
- **Interoperability**: Output can be processed by CASE-aware tools
- **Standards**: Aligns with DFRWS and CDO standards

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

- CASE schema validation
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
