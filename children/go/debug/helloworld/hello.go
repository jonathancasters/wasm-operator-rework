package main

import (
	"fmt"
	"time"
)

func main() {
	fmt.Println("Hello, world!")
	start := time.Now()
	time.Sleep(100 * time.Millisecond)
	fmt.Printf("Napped for %v\n", time.Since(start))
}

