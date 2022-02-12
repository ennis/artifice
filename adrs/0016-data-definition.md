# 14. Data definition language

Date: 2022-02-09

## Status

Draft

## Context

Serializing persistent data is annoying. In rust, if the data is both authored and consumed by the same application, then
we can get away with things like serde, or ad-hoc serialization code.

However, as soon as we want to manipulate this data from different applications (e.g. an editor and an engine), we need
to write a shared _I/O library_ to handle serialization of this data. This library is in charge of validating the _schema_
of the data and provide an API based on this schema to access the data (either by creating an in-memory representation or something else).
If the editor and the engine are not written in the same language, then we need to write the I/O library in the two languages
which doubles the maintainance burden. 
Then repeat this process for all kinds of structured data in the project.

Another challenge is creating tools to quickly create the actual _records_ of the structured data: most DDLs have tools to
generate the corresponding data structures in target languages, but nothing to help with _authoring_ the data.

## Examples of structured data used throughout artifice
Here are a few:
- styling data for kyute: contains colors, paints and border styles for kyute widgets
- artifice document format: nodes, operators, properties, etc.

## Terminology
- Storage format: how the data is stored on-disk
- Schema: description of the structure of the data

## Previous work

- [TableGen](https://llvm.org/docs/TableGen/) in LLVM: used for all kinds of things; gives a good overview of why it's valuable:
    
    > TableGen’s purpose is to help a human develop and maintain records of domain-specific information. 
    > Because there may be a large number of these records, it is specifically designed to allow writing flexible descriptions 
    > and for common features of these records to be factored out. This reduces the amount of duplication in the description, 
    > reduces the chance of error, and makes it easier to structure domain specific information. 

- [SmSchema](https://www.gdcvault.com/play/1026345/The-Future-of-Scene-Description): format used in God of War
- USD, in some ways
- see also: https://github.com/TheToolsmiths/ddl/wiki/Existing-DDL-formats


## Conclusion
Too much work right now (or rather, too many unknowns and questions lurking in the shadows, and we don't currently have any concrete workload).
Creating a custom language for describing styles in kyute is already a moderately big undertaking (parsing, export to json, load, ) 

=> right now, schemas are defined "implicitly" by the parser, and through documentation; there's no real validation