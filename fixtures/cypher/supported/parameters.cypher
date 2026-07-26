MATCH (n:Function) WHERE n.name = $name RETURN n LIMIT $limit
