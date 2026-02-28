In the "deploy" stage's first step (which has name "build"), change the command from `docker build -t app:latest .` back to `docker build -t app .`.

Challenge: There are TWO different things named "build" - a stage and a step. Navigate to the correct one in the array!
