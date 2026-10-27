paper-readiness-history = السجل التجريبي
paper-readiness-history-met =
    { $count ->
        [zero] لا جولات تجريبية مغلقة ({ $count })، المطلوب: { $needed }
        [one] جولة تجريبية مغلقة واحدة ({ $count })، المطلوب: { $needed }
        [two] جولتان تجريبيتان مغلقتان ({ $count })، المطلوب: { $needed }
        [few] { $count } جولات تجريبية مغلقة، المطلوب: { $needed }
        [many] { $count } جولة تجريبية مغلقة، المطلوب: { $needed }
       *[other] { $count } جولة تجريبية مغلقة، المطلوب: { $needed }
    }
paper-readiness-history-short = الجولات التجريبية المغلقة: { $count } من { $needed }
paper-readiness-profit = مربحة في التجريبي
paper-readiness-profit-detail =
    { $count ->
        [zero] { $realized } { -sol } محققة خلال { $count } جولة، الرابحة: { $wins }
        [one] { $realized } { -sol } محققة خلال جولة واحدة ({ $count })، الرابحة: { $wins }
        [two] { $realized } { -sol } محققة خلال جولتين ({ $count })، الرابحة: { $wins }
        [few] { $realized } { -sol } محققة خلال { $count } جولات، الرابحة: { $wins }
        [many] { $realized } { -sol } محققة خلال { $count } جولة، الرابحة: { $wins }
       *[other] { $realized } { -sol } محققة خلال { $count } جولة، الرابحة: { $wins }
    }
paper-readiness-exits = عمليات الخروج تُنفَّذ
paper-readiness-exits-ok = عمليات الخروج التجريبية الأخيرة وجدت تنفيذًا
paper-readiness-exits-failing = عمليات خروج تجريبية متتالية دون تنفيذ: { $count }
paper-live-block-setup-incomplete = أكمل إعداد المحفظة وRPC أولًا
paper-live-block-force-stop = الإيقاف الطارئ مفعّل
