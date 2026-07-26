MATCH (n:Function) WITH n.language AS language, n WHERE language = "rust" RETURN n.name
