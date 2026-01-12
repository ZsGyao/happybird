# -*- coding: utf-8 -*-
import datetime
import random

import pandas as pd

# =================配置区域=================
NUM_ROWS = 200  # 你想要生成的行数，建议 5000-10000 来测试性能
OUTPUT_FILE = "large_test_data.xlsx"
# =========================================

# 数据源素材
family_names = list(
    "赵钱孙李周吴郑王冯陈褚卫蒋沈韩杨朱秦尤许何吕施张孔曹严华金魏陶姜戚谢邹喻水云苏潘葛奚范彭郎鲁韦昌马苗凤花方俞任袁柳酆鲍史唐费廉岑薛雷贺倪汤滕殷罗毕郝邬安常乐于时傅皮卞齐康伍余元卜顾孟平黄和穆萧尹姚邵湛汪祁毛禹狄米贝明臧计伏成戴谈宋茅庞熊纪舒屈项祝董梁杜阮蓝闵席季麻强贾路娄危江童颜郭梅盛林刁钟徐邱骆高夏蔡田樊胡凌霍万柯卢莫房缪干解应宗丁宣邓郁单杭洪包诸左石崔吉钮龚"
)
given_names = list(
    "伟刚勇毅俊峰强军平保东文辉力明永健世广志义兴良海山仁波宁贵福生龙元全国胜学祥才发武新利清飞彬富顺信子杰涛昌成康星光天达安岩中茂进林有坚和彪博诚先敬震振壮会思群豪心邦承乐绍功松善厚庆磊民友裕河哲江超浩亮政谦亨奇固之轮翰朗伯宏言若鸣朋斌梁栋维启克伦翔旭鹏泽晨辰士以建家致树炎德行时泰盛雄琛钧冠策腾楠榕风航弘"
)
departments = [
    "研发部",
    "市场部",
    "财务部",
    "人事部",
    "运营部",
    "设计部",
    "销售部",
    "法务部",
    "总经办",
    "客服部",
]
positions_base = ["专员", "助理", "经理", "总监", "实习生", "组长", "高级专家"]
remarks_pool = [
    "技术栈主要用 Rust",
    "负责华东区业务",
    "擅长扁平化设计",
    "",
    "已离职",
    "",
    "细心负责",
    "会七十二变",
    "考勤全勤",
    "正在休产假",
    "年度优秀员工",
    "",
]


def get_random_name():
    """生成 2-3 个字的中文名"""
    name = random.choice(family_names) + random.choice(given_names)
    if random.random() > 0.3:  # 70% 概率是三个字
        name += random.choice(given_names)
    return name


def get_random_date(start_year=2015, end_year=2024):
    """生成随机日期字符串"""
    start = datetime.date(start_year, 1, 1)
    end = datetime.date(end_year, 12, 31)
    days_between = (end - start).days
    random_days = random.randrange(days_between)
    return (start + datetime.timedelta(days=random_days)).strftime("%Y-%m-%d")


def get_random_position(dept):
    """根据部门稍微调整职位名称"""
    prefix = ""
    if dept == "研发部":
        prefix = random.choice(["前端", "后端", "全栈", "测试", "运维"])
    elif dept == "设计部":
        prefix = random.choice(["UI", "UX", "视觉", "平面"])
    return f"{prefix}{random.choice(positions_base)}"


# === 开始生成 ===
print(f"正在生成 {NUM_ROWS} 条数据，请稍候...")

data = {
    "姓名": [],
    "年龄": [],
    "部门": [],
    "职位": [],
    "入职日期": [],
    "绩效评分": [],
    "是否在职": [],
    "备注": [],
    "薪水": [],
}

for i in range(NUM_ROWS):
    dept = random.choice(departments)

    data["姓名"].append(get_random_name())
    data["年龄"].append(random.randint(22, 60))
    data["部门"].append(dept)
    data["职位"].append(get_random_position(dept))
    data["入职日期"].append(get_random_date())
    data["绩效评分"].append(round(random.uniform(60.0, 100.0), 1))
    # 90% 概率在职
    data["是否在职"].append(True if random.random() < 0.9 else False)
    data["备注"].append(random.choice(remarks_pool))
    data["薪水"].append(round(random.uniform(4000.0, 100000.0), 1))

df = pd.DataFrame(data)
df.to_excel(OUTPUT_FILE, index=False)

print(f"✅ 成功！已生成文件：{OUTPUT_FILE}")
print(f"   包含列：{list(data.keys())}")
