// Build-only bridge to SWORD 1.9.0. Raw modules never ship as application code.
#include <swmgr.h>
#include <swmodule.h>
#include <versekey.h>
#include <listkey.h>
#include <swversion.h>
#include <iostream>
#include <regex>
#include <string>
#include <unordered_map>
using namespace sword;
static bool latin1 = false;
static std::string json(const std::string &s) {
    std::string out = "\"";
    const char *hex = "0123456789abcdef";
    for (unsigned char c : s) {
        if (c == '"' || c == '\\') { out += '\\'; out += c; }
        else if (c < 32) { out += "\\u00"; out += hex[c >> 4]; out += hex[c & 15]; }
        else if (latin1 && c >= 128) { out += static_cast<char>(0xc0 | (c >> 6)); out += static_cast<char>(0x80 | (c & 63)); }
        else out += c;
    }
    return out + '"';
}
int main(int argc, char **argv) {
    if (std::string(SWORD_VERSION_STR) != "1.9.0") return 2;
    if (argc == 2 && std::string(argv[1]) == "--version") {
        std::cout << "fontes-sword-export-v1/sword-1.9.0\n"; return 0;
    }
    if (argc != 3) return 2;
    SWMgr manager(argv[1], true, nullptr, false, false);
    SWModule *module = manager.getModule(argv[2]);
    if (!module) return 3;
    latin1 = module->getEncoding() != ENC_UTF8;
    module->setSkipConsecutiveLinks(false);
    auto *verse = dynamic_cast<VerseKey *>(module->getKey());
    if (verse) verse->setIntros(true);
    const std::regex refs(R"(<(reference|scripRef|ref)\b([^>]*)>([\s\S]*?)</\1>)");
    const std::regex judges(R"(\bJud(?=\s|[0-9]))");
    const std::regex attr(R"xx((osisRef|passage|target)="([^"]*)")xx");
    std::unordered_map<std::string, std::string> seen;
    std::unordered_map<std::string, unsigned> keyOccurrences;
    module->setPosition(TOP);
    for (int count = 0; !module->popError(); module->increment()) {
        if (++count > 100000) return 4;
        std::string key = verse ? verse->getOSISRef() : module->getKeyText();
        if (verse && key.empty()) key = "__testament." + std::to_string(verse->getTestament());
        unsigned occurrence = ++keyOccurrences[key];
        if (occurrence > 1) key += "#" + std::to_string(occurrence);
        if (!module->hasEntry(module->getKey())) continue;
        std::string raw = module->getRawEntry();
        if (raw.empty()) continue;
        auto found = seen.find(raw);
        if (found != seen.end() && std::string(argv[2]) != "TSK") {
            std::cout << "{\"key\":" << json(key) << ",\"same_as\":" << json(found->second) << "}\n";
            continue;
        }
        seen.emplace(raw, key);
        std::cout << "{\"key\":" << json(key) << ",\"raw\":" << json(raw)
                  << ",\"citations\":[";
        bool first = true;
        for (auto it = std::sregex_iterator(raw.begin(), raw.end(), refs); it != std::sregex_iterator(); ++it) {
            std::string tag = (*it)[1], attributes = (*it)[2], value = (*it)[3], kind;
            std::smatch match;
            if (std::regex_search(attributes, match, attr)) { kind = match[1]; value = match[2]; }
            else if (tag != "scripRef") continue;
            if (!first) std::cout << ',';
            first = false;
            std::cout << "{\"source\":" << json(value) << ",\"kind\":" << json(kind) << ",\"ranges\":[";
            // OSIS references are validated without normalization by Rust. SWORD
            // resolves ThML's abbreviated references against the owning verse.
            if (kind != "osisRef" && kind != "target") {
                VerseKey parser;
                parser.setLocale("en");
                parser.setAutoNormalize(false);
                std::string disambiguated = std::regex_replace(value, judges, "Judg");
                ListKey list = parser.parseVerseList(disambiguated.c_str(), key.c_str(), true);
                for (int i = 0; i < list.getCount(); ++i) {
                    auto *range = dynamic_cast<VerseKey *>(list.getElement(i));
                    if (!range) return 5;
                    if (i) std::cout << ',';
                    std::string lower = range->getLowerBound().getOSISRef();
                    std::string upper = range->getUpperBound().getOSISRef();
                    std::cout << '[' << json(lower) << ',' << json(upper) << ']';
                }
            }
            std::cout << "]}";
        }
        std::cout << "]}\n";
    }
    return 0;
}
