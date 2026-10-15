package com.asmolabs.vellum.plugin.builtin.translator;

import org.jsoup.Jsoup;
import org.jsoup.nodes.Document;
import org.jsoup.nodes.Element;
import org.jsoup.nodes.Node;
import org.jsoup.nodes.TextNode;
import org.jsoup.parser.Parser;

import java.util.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class EpubHtmlParser {

    public static final Set<String> BLOCK_TAGS = Set.of(
            "p", "h1", "h2", "h3", "h4", "h5", "h6", "div", "blockquote", "li", "td", "th",
            "section", "article", "aside", "header", "footer", "pre", "dt", "dd", "title",
            "caption", "figcaption"
    );

    private static final Pattern LETTER_PATTERN = Pattern.compile("[A-Za-zÀ-ÿ]");
    private static final Pattern SENTENCE_SPLIT_PATTERN = Pattern.compile("[^.!?]+[.!?]+(?:\\s+|$)|[^.!?]+(?:\\s+|$)");

    public record Context(String before, String after) {}

    public static boolean shouldTranslate(String text) {
        if (text == null) return false;
        String trimmed = text.trim();
        return !trimmed.isEmpty() && LETTER_PATTERN.matcher(trimmed).find();
    }

    public static Document parseHtml(String html) {
        Document doc = Jsoup.parse(html, "", Parser.xmlParser());
        doc.outputSettings().prettyPrint(false);
        doc.outputSettings().syntax(Document.OutputSettings.Syntax.xml);
        return doc;
    }

    public static List<TextNode> collectTextNodes(Node root) {
        List<TextNode> nodes = new ArrayList<>();
        collectTextNodesRecursive(root, nodes);
        return nodes;
    }

    private static void collectTextNodesRecursive(Node node, List<TextNode> destination) {
        if (node == null) return;
        if (node instanceof TextNode tn) {
            Node parent = tn.parent();
            String parentName = parent != null ? parent.nodeName().toLowerCase() : "";
            if (!"script".equals(parentName) && !"style".equals(parentName)) {
                destination.add(tn);
            }
            return;
        }

        for (Node child : node.childNodes()) {
            collectTextNodesRecursive(child, destination);
        }
    }

    public static Element findBlockAncestor(Node node) {
        Node cur = node.parent();
        while (cur != null) {
            if (cur instanceof Element elem) {
                String name = elem.normalName().toLowerCase();
                if (BLOCK_TAGS.contains(name) || "body".equals(name) || "html".equals(name)) {
                    return elem;
                }
            }
            cur = cur.parent();
        }
        return null;
    }

    public static Context getContext(List<TextNode> nodes, int index, int maxLen) {
        if (index < 0 || index >= nodes.size()) return new Context("", "");

        TextNode current = nodes.get(index);
        Element currentBlock = findBlockAncestor(current);

        StringBuilder before = new StringBuilder();
        int idxBefore = index - 1;
        while (idxBefore >= 0 && before.length() < maxLen) {
            TextNode cand = nodes.get(idxBefore);
            if (findBlockAncestor(cand) != currentBlock) break;
            String t = cand.text().trim();
            if (!t.isEmpty()) {
                before.insert(0, t + " ");
            }
            idxBefore--;
        }

        StringBuilder after = new StringBuilder();
        int idxAfter = index + 1;
        while (idxAfter < nodes.size() && after.length() < maxLen) {
            TextNode cand = nodes.get(idxAfter);
            if (findBlockAncestor(cand) != currentBlock) break;
            String t = cand.text().trim();
            if (!t.isEmpty()) {
                after.append(t).append(" ");
            }
            idxAfter++;
        }

        String bStr = before.toString().trim();
        if (bStr.length() > maxLen) {
            bStr = bStr.substring(bStr.length() - maxLen);
        }

        String aStr = after.toString().trim();
        if (aStr.length() > maxLen) {
            aStr = aStr.substring(0, maxLen);
        }

        return new Context(bStr, aStr);
    }

    public static List<String> splitTextIntoChunks(String text, int maxLen) {
        List<String> sentences = new ArrayList<>();
        Matcher m = SENTENCE_SPLIT_PATTERN.matcher(text);
        while (m.find()) {
            String s = m.group().trim();
            if (!s.isEmpty()) sentences.add(s);
        }
        if (sentences.isEmpty()) {
            sentences.add(text.trim());
        }

        List<String> chunks = new ArrayList<>();
        StringBuilder current = new StringBuilder();

        for (String sentence : sentences) {
            if (current.length() == 0) {
                current.append(sentence);
            } else if (current.length() + 1 + sentence.length() <= maxLen) {
                current.append(" ").append(sentence);
            } else {
                chunks.add(current.toString());
                current.setLength(0);
                if (sentence.length() > maxLen) {
                    String rest = sentence;
                    while (rest.length() > maxLen) {
                        int cut = rest.lastIndexOf(' ', maxLen);
                        if (cut <= 0) cut = maxLen;
                        chunks.add(rest.substring(0, cut).trim());
                        rest = rest.substring(cut).trim();
                    }
                    current.append(rest);
                } else {
                    current.append(sentence);
                }
            }
        }

        if (current.length() > 0) {
            chunks.add(current.toString());
        }

        return chunks;
    }

    public static String preserveWhitespace(String original, String translated) {
        Matcher leadMatcher = Pattern.compile("^\\s*").matcher(original);
        String lead = leadMatcher.find() ? leadMatcher.group() : "";

        Matcher trailMatcher = Pattern.compile("\\s*$").matcher(original);
        String trail = trailMatcher.find() ? trailMatcher.group() : "";

        return lead + translated.trim() + trail;
    }

    public static String extractVisibleText(String html) {
        Document doc = Jsoup.parse(html);
        Element body = doc.body();
        return (body != null ? body.text() : doc.text()).replaceAll("\\s+", " ").trim();
    }
}
