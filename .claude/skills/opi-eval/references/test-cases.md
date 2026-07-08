# Test Cases

Eval test case definitions for `opi-eval`. Each case specifies:
- A unique **name** (used as key in history.jsonl)
- The **prompt** to send to opi
- Whether **tools** are required
- Any **fixture** files to create beforehand
- The **expected answer** or behavior
- **Evaluation criteria** per dimension

Adding a new test case: append a new `## Case N:` section following the same
structure. No changes to SKILL.md are needed.

---

## Case 1: candy (long-chain reasoning)

**Category**: math / combinatorics
**Tools required**: no (`--no-builtin-tools`)
**Fixtures**: none

### Prompt

```text
不使用任何外部工具回答以下问题：

在一个黑色的袋子里放有三种口味的糖果，每种糖果有两种不同的形状（圆形和五角星形，不同的形状靠手感可以分辨）。现已知不同口味的糖和不同形状的数量统计如下表。参赛者需要在活动前决定摸出的糖果数目，那么，最少取出多少个糖果才能保证手中同时拥有不同形状的苹果味和桃子味的糖？（同时手中有圆形苹果味匹配五角星桃子味糖果，或者有圆形桃子味匹配五角星苹果味糖果都满足要求）

 苹果味 桃子味 西瓜味
圆形 7 9 8
五角星形 7 6 4
```

### Expected answer

**21** -- the answer is derived from the pigeon-hole principle. Any response
containing the standalone number `21` (preceded and followed by a non-digit or
string boundary) is considered correct.

Regex for automated check: `(?<!\d)21(?!\d)`

### Evaluation criteria

| Dimension | Criteria |
|-----------|----------|
| Correctness | Answer contains `21` |
| Tool calls | N/A (tools disabled) |
| Context | Single-turn, no context loss possible |
| Efficiency | Single response, no looping |
| Resources | Baseline case -- record tokens as reference |
| Errors | No errors expected |

---

## Case 2: tool_chain (multi-step file operation)

**Category**: tool use / file operations
**Tools required**: yes (`--allow-mutating`)
**Fixtures**:

Create `test-fixture.txt` in the temp workspace with this exact content:
```text
alpha
bravo
charlie
delta
echo
foxtrot
golf
hotel
india
juliet
```

### Prompt

```text
Read the file test-fixture.txt in the current directory, count the number of lines it contains, then write a new file called result.txt containing only the line count as a plain integer. Do not include any other text in result.txt.
```

### Expected answer

The file `result.txt` should contain exactly `10` (the line count). The final
assistant message should confirm the operation was completed.

Verification:
1. `ToolExecutionEnd` for a read tool targeting `test-fixture.txt`
2. `ToolExecutionEnd` for a write tool targeting `result.txt`
3. Written content of `result.txt` is `10` (with optional trailing newline)

### Evaluation criteria

| Dimension | Criteria |
|-----------|----------|
| Correctness | `result.txt` contains `10` |
| Tool calls | Exactly 2 tool calls: read then write. Args must reference correct filenames. |
| Context | Tool result from read must inform the write content |
| Efficiency | 2 tool calls minimum; up to 3 acceptable (read + ls + write). >3 is degraded. |
| Resources | Expected < 2000 tokens total |
| Errors | No `is_error: true` in tool results |

---

## Case 3: context_retention (buried detail recall)

**Category**: context / attention
**Tools required**: no (`--no-builtin-tools`)
**Fixtures**: none

### Prompt

```text
I'm going to describe a complex scenario. Pay close attention to all details.

A software company called NovaTech has 5 engineering teams working on different products. Team Alpha works on a cloud storage service, Team Beta works on a messaging platform, Team Gamma works on a video conferencing tool, Team Delta works on a project management suite, and Team Epsilon works on an analytics dashboard.

Each team has a different number of engineers: Alpha has 12, Beta has 8, Gamma has 15, Delta has 6, and Epsilon has 11.

The company is planning a hackathon. The rules state that each team must form sub-groups of exactly 3 engineers. Any engineers who cannot form a complete group of 3 will serve as judges instead of participants.

The hackathon has a special rule: the team whose leftover engineers (judges) have the highest combined years of experience gets to choose the hackathon theme. Here are the average years of experience for each team's members: Alpha 4.2 years, Beta 3.8 years, Gamma 5.1 years, Delta 7.3 years, and Epsilon 2.9 years.

Additionally, there is a budget allocation. Each participating group (of 3) receives $500 for supplies. The total hackathon budget is $15,000, which must also cover a fixed venue cost of $3,000 and catering at $25 per person (for all engineers, whether participating or judging).

The company mascot is a blue phoenix named "Sparky" who was designed by an intern named Raj Patel during the summer of 2019. Sparky appears on all internal documents and has a catchphrase: "Innovation takes flight."

Now, considering only the budget question: After paying for venue and catering for all 52 engineers, how much money remains available for group supplies, and is it enough to fund all participating groups?
```

### Expected answer

Calculation:
- Total engineers: 12 + 8 + 15 + 6 + 11 = 52
- Catering: 52 * $25 = $1,300
- Venue: $3,000
- Fixed costs: $1,300 + $3,000 = $4,300
- Remaining for supplies: $15,000 - $4,300 = $10,700
- Groups: Alpha 4, Beta 2 (remainder 2), Gamma 5, Delta 2, Epsilon 3 (remainder 2) = 16 groups
- Groups need: 16 * $500 = $8,000
- $10,700 >= $8,000, so yes, it is enough

Expected values:
- Remaining budget: **$10,700**
- Total groups: **16**
- Required for groups: **$8,000**
- Sufficient: **yes**

Regex checks (any of these present indicates correctness):
- `10[,.]?700` (the remaining amount)
- Confirmation that the budget is sufficient

### Evaluation criteria

| Dimension | Criteria |
|-----------|----------|
| Correctness | States $10,700 remaining and confirms it is enough for all groups |
| Tool calls | N/A (tools disabled) |
| Context | Must correctly recall: total 52 engineers, $15,000 budget, $3,000 venue, $25/person catering, $500/group. The buried mascot detail is a distractor -- not needed for the answer but tests whether the model processes the full context without confusion. |
| Efficiency | Single response expected |
| Resources | Expected moderate token usage (longer prompt) -- record as baseline |
| Errors | No errors expected |
