from sentence_transformers import SentenceTransformer, util
import torch

embedder = SentenceTransformer('msmarco-MiniLM-L12-cos-v5')

#Our sentences we like to encode
corpus = [
    """Set on the desert planet Arrakis, Dune is the story of the boy Paul Atreides, heir to a noble family tasked with ruling an inhospitable world where the only thing of value is the "spice" melange, a drug capable of extending life and enhancing consciousness. Coveted across the known universe, melange is a prize worth killing for...
When House Atreides is betrayed, the destruction of Paul's family will set the boy on a journey toward a destiny greater than he could ever have imagined. And as he evolves into the mysterious man known as Muad'Dib, he will bring to fruition humankind's most ancient and unattainable dream.

A stunning blend of adventure and mysticism, environmentalism and politics, Dune won the first Nebula Award, shared the Hugo Award, and formed the basis of what is undoubtedly the grandest epic in science fiction.
    """,
    """Ender's Game is a 1985 military science fiction novel by American author Orson Scott Card. Set at an unspecified date in Earth's future, the novel presents an imperiled humankind after two conflicts with the Formics, an insectoid alien species they dub the "buggers". In preparation for an anticipated third invasion, children, including the novel's protagonist, Andrew "Ender" Wiggin, are trained from a very young age by putting them through increasingly difficult games, including some in zero gravity, where Ender's tactical genius is revealed.
The book originated as a short story of the same name, published in the August 1977 issue of Analog Science Fiction and Fact. The novel was published on January 15, 1985. Later, by elaborating on characters and plotlines depicted in the novel, Card was able to write additional books in the Ender's Game series. Card also released an updated version of Ender's Game in 1991, changing some political facts to reflect the times more accurately (e.g., to include the recent collapse of the Soviet Union and the end of the Cold War). The novel has been translated into 34 languages.

Reception of the book has been mostly positive. It has become suggested reading for many military organizations, including the United States Marine Corps. Ender's Game was recognized as "best novel" by the 1985 Nebula Award[3] and the 1986 Hugo Award[4] in the genres of science fiction and fantasy. Its four sequels—Speaker for the Dead (1986), Xenocide (1991), Children of the Mind (1996), and Ender in Exile (2008)—follow Ender's subsequent travels to many different worlds in the galaxy. In addition, the later novella A War of Gifts (2007) and novel Ender's Shadow (1999), plus other novels in the Shadow saga, take place during the same time period as the original.
    """
]

corpus_embeddings = embedder.encode(corpus, convert_to_tensor=True)

top_k = min(1, len(corpus))
query_embedding = embedder.encode("science fiction with kids in war games", convert_to_tensor=True)

# We use cosine-similarity and torch.topk to find the highest 5 scores
cos_scores = util.cos_sim(query_embedding, corpus_embeddings)[0]
top_results = torch.topk(cos_scores, k=top_k)

"""
print("\n\n======================\n\n")
print("\nTop 1 most similar sentences in corpus:")

for score, idx in zip(top_results[0], top_results[1]):
    print(corpus[idx], "(Score: {:.4f})".format(score))
"""

# Alternatively, we can also use util.semantic_search to perform cosine similarty + topk
hits = util.semantic_search(query_embedding, corpus_embeddings, top_k=1)
hits = hits[0]      #Get the hits for the first query
for hit in hits:
    print(corpus[hit['corpus_id']], "(Score: {:.4f})".format(hit['score']))

