import Foundation

// Usage: gen <corpus file> <query>...
// For each query: every path FuzzyScorer.scoreFile matches, ordered by score
// descending and then path ascending (the reference's sort is unstable, so
// ties are broken by path to make the snapshot deterministic), top 50 - the
// reference's maxResults. Output: JSON lines {query, path, score, indices}.
let args = CommandLine.arguments
let corpus = try! String(contentsOfFile: args[1], encoding: .utf8)
    .components(separatedBy: "\n").filter { !$0.isEmpty }
for query in args.dropFirst(2) {
    let scored = corpus.compactMap { path -> (String, FuzzyScorer.ScoredMatch)? in
        guard let m = FuzzyScorer.scoreFile(pattern: query, path: path) else { return nil }
        return (path, m)
    }.sorted { $0.1.score != $1.1.score ? $0.1.score > $1.1.score : $0.0 < $1.0 }
    for (path, m) in scored.prefix(50) {
        let obj: [String: Any] = ["query": query, "path": path, "score": m.score, "indices": m.matchedIndices]
        let data = try! JSONSerialization.data(withJSONObject: obj, options: [.sortedKeys])
        print(String(data: data, encoding: .utf8)!)
    }
}
