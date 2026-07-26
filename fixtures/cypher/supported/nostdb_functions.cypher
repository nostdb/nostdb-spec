MATCH (n:Function) RETURN nostdb.source(n) AS source, nostdb.source_location(n) AS path, nostdb.source_revision(n) AS revision, nostdb.link_alias(n) AS alias, nostdb.is_available(n) AS available
