MATCH (a:Service) RETURN a.name UNION MATCH (b:Database) RETURN b.title
