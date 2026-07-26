MATCH (a:Function) OPTIONAL MATCH (a)-[:CALLS]->(b:Database) RETURN a.name, b.name
